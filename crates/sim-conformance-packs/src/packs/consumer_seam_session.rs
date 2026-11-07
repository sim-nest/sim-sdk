// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Outside session state for the consumer-seams specimen.

use std::collections::BTreeMap;

use super::{
    Advice, Edge, Export, MAX_SOURCES, PAGE_ROWS, PROVIDERS, ProofView, REQUIRED_LEAF, Request,
    SeamFault, Source, digest,
};

impl super::Session {
    pub(crate) fn open(providers: &[&str]) -> Result<Self, SeamFault> {
        let mut used = BTreeMap::new();
        for provider in providers {
            if provider.contains("linux") {
                return Err(SeamFault::LinuxProvider);
            }
            used.insert((*provider).to_owned(), false);
        }
        let mut rows = BTreeMap::new();
        rows.insert(
            1,
            vec![
                ("fact/alpha".to_owned(), "a".to_owned()),
                ("fact/beta".to_owned(), "b".to_owned()),
                ("fact/gamma".to_owned(), "c".to_owned()),
                ("fact/delta".to_owned(), "d".to_owned()),
            ],
        );
        let mut affordances = BTreeMap::new();
        affordances.insert("affordance/review".to_owned(), "review".to_owned());
        Ok(Self {
            live: 1,
            rows,
            request: None,
            consumed: None,
            transitions: 0,
            effects: 0,
            begin: false,
            selected: false,
            candidate: String::new(),
            sources: BTreeMap::new(),
            edges: Vec::new(),
            landing: String::new(),
            providers: used,
            pending: false,
            revoked: BTreeMap::new(),
            affordances,
        })
    }

    pub(crate) fn touch(&mut self, provider: &str) -> Result<(), SeamFault> {
        let slot = self
            .providers
            .get_mut(provider)
            .ok_or(SeamFault::MissingObject)?;
        *slot = true;
        Ok(())
    }

    pub(crate) fn advance(&mut self) {
        let next = self.live + 1;
        let rows = self.rows.get(&self.live).cloned().unwrap_or_default();
        self.rows.insert(next, rows);
        self.live = next;
    }

    pub(crate) fn read_page(
        &mut self,
        held: u64,
        cursor: &mut usize,
    ) -> Result<Vec<(String, String)>, SeamFault> {
        self.touch("model/snapshot")?;
        if held != self.live {
            return Err(SeamFault::RevisionMoved { retained: held });
        }
        let rows = self.rows.get(&held).ok_or(SeamFault::MissingObject)?;
        if *cursor >= rows.len() {
            return Err(SeamFault::MissingObject);
        }
        let end = (*cursor + PAGE_ROWS).min(rows.len());
        let page = rows[*cursor..end].to_vec();
        *cursor = end;
        Ok(page)
    }

    pub(crate) fn resolve_evidence(&mut self, held: u64, id: &str) -> Result<String, SeamFault> {
        self.touch("model/snapshot")?;
        if held != self.live {
            return Err(SeamFault::RevisionMoved { retained: held });
        }
        self.rows
            .get(&held)
            .and_then(|rows| {
                rows.iter()
                    .find(|(key, _)| key == id)
                    .map(|(_, value)| value.clone())
            })
            .ok_or(SeamFault::MissingObject)
    }

    pub(crate) fn ask(&mut self) -> Result<Request, SeamFault> {
        self.touch("model/direction")?;
        let request = Request {
            execution: "execution/outside".to_owned(),
            revision: self.live,
            answer: format!("approve-{}", self.live),
        };
        self.request = Some(Request {
            execution: request.execution.clone(),
            revision: request.revision,
            answer: request.answer.clone(),
        });
        Ok(request)
    }

    pub(crate) fn reply(
        &mut self,
        execution: &str,
        revision: u64,
        answer: &str,
        shape: &str,
    ) -> Result<(), SeamFault> {
        self.touch("model/direction")?;
        if shape != "decision" {
            return Err(SeamFault::AdviceInDecision);
        }
        let request = self.request.as_ref().ok_or(SeamFault::MissingObject)?;
        if execution != request.execution {
            return Err(SeamFault::WrongExecution);
        }
        if answer != request.answer {
            return Err(SeamFault::ChangedAnswer);
        }
        if revision != self.live || revision != request.revision {
            return Err(SeamFault::StaleDirection);
        }
        if self.consumed.as_deref() == Some(answer) {
            return Err(SeamFault::Replayed);
        }
        self.consumed = Some(answer.to_owned());
        self.transitions += 1;
        Ok(())
    }

    pub(crate) fn note_source(&mut self, id: &str, body: &str) -> Result<(), SeamFault> {
        if reserved_id(id)
            || id == self.candidate
            || self.edges.iter().any(|edge| edge.commission == id)
        {
            return Err(SeamFault::MissingObject);
        }
        if self.sources.contains_key(id) {
            return self.revise_source(id, body);
        }
        if self.sources.len() >= MAX_SOURCES {
            return Err(SeamFault::Bound);
        }
        self.sources.insert(
            id.to_owned(),
            Source {
                digest: digest(body.as_bytes()),
            },
        );
        Ok(())
    }

    pub(crate) fn note_commission(&mut self, id: &str, source: &str) -> Result<(), SeamFault> {
        if reserved_id(id)
            || id == self.candidate
            || self.sources.contains_key(id)
            || self.edges.iter().any(|edge| edge.commission == id)
        {
            return Err(SeamFault::MissingObject);
        }
        if !self.sources.contains_key(source) {
            return Err(SeamFault::MissingObject);
        }
        self.edges.push(Edge {
            commission: id.to_owned(),
            source: source.to_owned(),
            stale: false,
        });
        Ok(())
    }

    pub(crate) fn select(&mut self, candidate: &str) -> Result<(), SeamFault> {
        if reserved_id(candidate)
            || self.sources.contains_key(candidate)
            || self.edges.iter().any(|edge| edge.commission == candidate)
        {
            return Err(SeamFault::MissingObject);
        }
        self.candidate = candidate.to_owned();
        self.selected = true;
        self.begin = false;
        Ok(())
    }

    pub(crate) fn grant_begin(&mut self) {
        self.begin = true;
    }

    pub(crate) fn execute(&mut self) -> Result<(), SeamFault> {
        if self.pending {
            return Err(SeamFault::Interrupted);
        }
        if !self.selected {
            return Err(SeamFault::MissingObject);
        }
        if !self.begin {
            return Err(SeamFault::MissingBegin);
        }
        self.touch("model/landing")?;
        self.landing = self.landing_digest();
        self.effects += 1;
        Ok(())
    }

    pub(crate) fn revise_source(&mut self, id: &str, body: &str) -> Result<(), SeamFault> {
        let source = self.sources.get_mut(id).ok_or(SeamFault::MissingObject)?;
        source.digest = digest(body.as_bytes());
        for edge in &mut self.edges {
            if edge.source == id {
                edge.stale = true;
            }
        }
        Ok(())
    }

    pub(crate) fn proof_key(&self, id: &str) -> Result<String, SeamFault> {
        self.sources
            .get(id)
            .map(|source| source.digest.clone())
            .ok_or(SeamFault::MissingObject)
    }

    pub(crate) fn proof_view(&self) -> ProofView {
        let mut leaves = vec![REQUIRED_LEAF.to_owned()];
        leaves.extend(self.sources.keys().cloned());
        let gaps = self
            .edges
            .iter()
            .filter(|edge| edge.stale)
            .map(|edge| format!("stale:{}", edge.source))
            .collect();
        ProofView {
            leaves,
            gaps,
            reasons: vec!["floor-holds".to_owned()],
            floor: "conservative".to_owned(),
            owner: Some("owner/outside".to_owned()),
        }
    }

    pub(crate) fn advise(&self, _view: &ProofView) -> Advice {
        Advice {
            shape: "advice".to_owned(),
            text: "labeled advice".to_owned(),
        }
    }

    pub(crate) fn attach_receipt(&self, advice: &Advice) -> Result<(), SeamFault> {
        let _ = advice;
        Err(SeamFault::AdviceRejected)
    }

    pub(crate) fn attach_decision(&self, advice: &Advice) -> Result<(), SeamFault> {
        let _ = advice;
        Err(SeamFault::AdviceRejected)
    }

    pub(crate) fn export(&self) -> Export {
        let mut sources = BTreeMap::new();
        for (id, source) in &self.sources {
            sources.insert(id.clone(), source.digest.clone());
        }
        let mut objects = BTreeMap::new();
        objects.insert("artifact/1".to_owned(), self.landing.clone());
        objects.insert("landing/1".to_owned(), self.landing.clone());
        for edge in &self.edges {
            objects.insert(edge.commission.clone(), edge.source.clone());
        }
        if !self.candidate.is_empty() {
            objects.insert(self.candidate.clone(), self.candidate.clone());
        }
        Export {
            objects,
            sources,
            consumed: self.consumed.clone(),
            landing: self.landing.clone(),
            edges: self.edges.clone(),
            live: self.live,
            selected: self.selected,
            begin: self.begin,
            candidate: self.candidate.clone(),
        }
    }

    pub(crate) fn reopen(export: &Export) -> Result<Self, SeamFault> {
        if export.objects.get("artifact/1").map(String::as_str) != Some(export.landing.as_str())
            || export.objects.get("landing/1").map(String::as_str) != Some(export.landing.as_str())
        {
            return Err(SeamFault::MissingObject);
        }
        if !export.candidate.is_empty() && reserved_id(&export.candidate) {
            return Err(SeamFault::MissingObject);
        }
        let mut sources = BTreeMap::new();
        for (id, value) in &export.sources {
            if reserved_id(id)
                || !is_source_digest(value)
                || id == &export.candidate
                || export
                    .edges
                    .iter()
                    .any(|edge| edge.commission == id.as_str())
            {
                return Err(SeamFault::MissingObject);
            }
            sources.insert(
                id.clone(),
                Source {
                    digest: value.clone(),
                },
            );
        }
        for edge in &export.edges {
            if reserved_id(&edge.commission) || reserved_id(&edge.source) {
                return Err(SeamFault::MissingObject);
            }
            let named = export
                .objects
                .get(&edge.commission)
                .ok_or(SeamFault::MissingObject)?;
            if named != &edge.source || !sources.contains_key(&edge.source) {
                return Err(SeamFault::MissingObject);
            }
        }
        if export.live == 0 {
            return Err(SeamFault::MissingObject);
        }
        let mut session = Self::open(PROVIDERS)?;
        session.sources = sources;
        session.consumed = export.consumed.clone();
        session.landing = export.landing.clone();
        session.edges = export.edges.clone();
        session.live = export.live;
        if export.live != 1 {
            let rows = session.rows.get(&1).cloned().unwrap_or_default();
            session.rows.insert(export.live, rows);
        }
        session.selected = export.selected;
        session.begin = export.begin;
        session.candidate = export.candidate.clone();
        if export.selected == export.candidate.is_empty() || (export.begin && !export.selected) {
            return Err(SeamFault::MissingObject);
        }
        if !export.candidate.is_empty()
            && export.objects.get(&export.candidate).map(String::as_str)
                != Some(export.candidate.as_str())
        {
            return Err(SeamFault::MissingObject);
        }
        if !session.landing.is_empty() && session.landing_digest() != session.landing {
            return Err(SeamFault::MissingObject);
        }
        Ok(session)
    }

    pub(crate) fn begin_effect(&mut self) -> Result<(), SeamFault> {
        self.pending = true;
        Ok(())
    }

    pub(crate) fn interrupt(&mut self) -> Result<(), SeamFault> {
        if !self.pending {
            return Err(SeamFault::MissingObject);
        }
        self.pending = false;
        Ok(())
    }

    pub(crate) fn revoke(&mut self, id: &str) {
        self.revoked.insert(id.to_owned(), true);
    }

    pub(crate) fn read_subject(&self, id: &str) -> Result<(), SeamFault> {
        if self.revoked.contains_key(id) {
            return Err(SeamFault::Revoked);
        }
        Ok(())
    }

    pub(crate) fn discover(&self, id: &str) -> Result<String, SeamFault> {
        self.affordances
            .get(id)
            .cloned()
            .ok_or(SeamFault::MissingObject)
    }

    pub(crate) fn hot_pass(&mut self) -> Result<bool, SeamFault> {
        if self.pending {
            return Err(SeamFault::Interrupted);
        }
        self.touch("model/landing")?;
        let next = self.landing_digest();
        if next == self.landing {
            return Ok(false);
        }
        if !self.selected {
            return Err(SeamFault::MissingObject);
        }
        if !self.begin {
            return Err(SeamFault::MissingBegin);
        }
        self.landing = next;
        self.effects += 1;
        Ok(true)
    }

    pub(crate) fn landing_digest(&self) -> String {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(self.sources.len() as u64).to_be_bytes());
        for (id, source) in &self.sources {
            push_frame(&mut bytes, id.as_bytes());
            push_frame(&mut bytes, source.digest.as_bytes());
        }
        let mut edges = self.edges.iter().collect::<Vec<_>>();
        edges.sort_by(|left, right| {
            (&left.commission, &left.source).cmp(&(&right.commission, &right.source))
        });
        bytes.extend_from_slice(&(edges.len() as u64).to_be_bytes());
        for edge in edges {
            push_frame(&mut bytes, edge.commission.as_bytes());
            push_frame(&mut bytes, edge.source.as_bytes());
        }
        push_frame(&mut bytes, self.candidate.as_bytes());
        digest(&bytes)
    }
}

fn reserved_id(id: &str) -> bool {
    id.is_empty()
        || id == "artifact/1"
        || id == "landing/1"
        || id
            .bytes()
            .any(|byte| matches!(byte, 0 | b'\n' | b'\r' | b':'))
}

fn push_frame(bytes: &mut Vec<u8>, field: &[u8]) {
    bytes.extend_from_slice(&(field.len() as u64).to_be_bytes());
    bytes.extend_from_slice(field);
}

fn is_source_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

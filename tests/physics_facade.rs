// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// conformance: the SDK physics facade validates an immutable boundary and event graph.

#![cfg(feature = "physics")]

use sim::physics::core::{
    Boundary, BoundaryClosure, BoundaryId, Endpoint, Event, EventGraph, EventRef, PhysicsError,
    PortRef, SHAPES, StateRef, StoreRef, TimeSpan, Transfer, TransferPath,
};
use sim_lib_numbers_quantity::{BaseDimension, Dimension, ExactScalar, MeasureRole, Quantity};

fn q(v: i64) -> sim::physics::core::PhysicalQuantity {
    Quantity::new(
        ExactScalar::from(v),
        Dimension::base(BaseDimension::Time),
        None,
        None,
        MeasureRole::Interval,
    )
    .unwrap()
}

fn graph() -> EventGraph {
    let boundary = Boundary {
        id: BoundaryId::new("tank").unwrap(),
        closure: BoundaryClosure::Open,
        stores: vec![StoreRef::new("thermal").unwrap()],
        ports: vec![PortRef::new("heater").unwrap()],
    };
    let event = Event {
        id: EventRef::new("switch-on").unwrap(),
        at: q(1),
        influences: vec![StateRef::new("temperature").unwrap()],
    };
    let transfer = Transfer {
        from: Endpoint::SpanStart,
        to: Endpoint::Event(event.id.clone()),
        path: TransferPath::Crossing {
            boundary: boundary.id.clone(),
            port: boundary.ports[0].clone(),
            store: boundary.stores[0].clone(),
        },
        signed_power: q(10),
    };
    let mut graph = EventGraph {
        id: String::new(),
        span: TimeSpan {
            start: q(0),
            end: q(2),
        },
        boundaries: vec![boundary],
        events: vec![event],
        transfers: vec![transfer],
    };
    graph.id = graph.canonical_id();
    graph
}

#[test]
fn sdk_physics_facade_validates_a_boundary_event_graph() {
    let graph = graph();
    graph.validate().unwrap();
    assert!(graph.read_construct().starts_with("#(physics/EventGraph"));
    assert_eq!(SHAPES.len(), 4);
}

#[test]
fn sdk_physics_facade_rejects_an_undeclared_crossing() {
    let mut graph = graph();
    if let TransferPath::Crossing { port, .. } = &mut graph.transfers[0].path {
        *port = PortRef::new("missing").unwrap();
    }
    graph.id = graph.canonical_id();
    assert!(matches!(
        graph.validate(),
        Err(PhysicsError::InvalidTransfer(_))
    ));
}

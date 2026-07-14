macro_rules! cookbook_directory_femm {
    ($m:ident) => {
        $m!(
            "femm-core",
            "FEMM core",
            "femm-core",
            Some(crate::femm_core::RECIPES),
            || Box::new(crate::femm_core::FemmCoreLib::new())
        );
        $m!(
            "femm-field",
            "FEMM field",
            "femm-field",
            Some(crate::femm_field::RECIPES),
            || Box::new(crate::femm_field::FemmFieldLib::new())
        );
        $m!(
            "femm-function",
            "FEMM function",
            "femm-function",
            Some(crate::femm_function::RECIPES),
            || Box::new(crate::femm_function::FemmFunctionLib::new())
        );
        $m!(
            "femm-ode",
            "FEMM ODE",
            "femm-ode",
            Some(crate::femm_ode::RECIPES),
            || Box::new(crate::femm_ode::FemmOdeLib::new())
        );
    };
}

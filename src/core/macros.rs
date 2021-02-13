#[macro_export]
macro_rules! do_internal(
    ($func:ident) => {
        Box::new(|model: &mut $crate::model::Model| {
            model.$func();
        }) as $crate::binding::KeyEvents
    };

    ($func:ident, $($arg:expr),+) => {
        Box::new(move |model: &mut $crate::model::Model| {
            model.$func($($arg),+);
        }) as $crate::binding::KeyEvents
    };
);

#[macro_export]
macro_rules! do_internal_block(
    ($model:ident, $body:block) => {
        Box::new(|$model: &mut $crate::model::Model| {
            $body
        }) as $crate::binding::KeyEvents
    };
);

#[macro_export]
macro_rules! spawn_external(
    ($cmd:expr) => {
        {
            Box::new(move |_: &mut $crate::model::Model| {
                $crate::util::Util::spawn($cmd);
            }) as $crate::binding::KeyEvents
        }
    };
);

#[macro_export]
macro_rules! spawn_from_shell(
    ($cmd:expr) => {
        {
            Box::new(move |_: &mut $crate::model::Model| {
                $crate::util::Util::spawn_shell($cmd);
            }) as $crate::binding::KeyEvents
        }
    };
);

#[macro_export]
macro_rules! build_key_bindings(
    { @start $key_bindings:expr, $keycodes:expr,
        $( $binding:expr ),+ => $action:expr,
        $($tail:tt)*
    } => {
        $(
            match $crate::util::Util::parse_key_binding($binding, &$keycodes) {
                None => panic!("could not parse key binding: {}", $binding),
                Some(keycode) => $key_bindings.insert(keycode, $action),
            };
        )+
        build_key_bindings!(@start $key_bindings, $keycodes, $($tail)*);
    };

    { @start $key_bindings:expr, $keycodes:expr,
        $($tail:tt)*
    } => {
        $(compile_error!(
            stringify!(incorrect syntax in build_key_bindings: $tail)
        );)*
    };

    { $($tokens:tt)+ } => {
        {
            let mut key_bindings = std::collections::HashMap::new();
            let keycodes = $crate::util::Util::system_keycodes();
            build_key_bindings!(@start key_bindings, keycodes, $($tokens)+);
            key_bindings
        }
    };
);

#[macro_export]
macro_rules! call(
    ($($method:tt)+) => {
        |arg| $($method)+(arg)
    };
);

#[macro_export]
macro_rules! do_internal(
    ($func:ident) => {
        Box::new(|model: &mut $crate::model::Model<'_>| {
            drop(model.$func());
        }) as $crate::binding::KeyAction
    };

    ($func:ident, $($arg:expr),+) => {
        Box::new(move |model: &mut $crate::model::Model<'_>| {
            drop(model.$func($($arg),+));
        }) as $crate::binding::KeyAction
    };
);

#[macro_export]
macro_rules! do_internal_block(
    ($model:ident, $body:block) => {
        Box::new(|$model: &mut $crate::model::Model<'_>| {
            $body
        }) as $crate::binding::KeyAction
    };
);

#[macro_export]
macro_rules! do_nothing(
    () => {
        Box::new(|_: &mut $crate::model::Model<'_>, _| {}) as $crate::binding::MouseAction
    };
);

#[macro_export]
macro_rules! do_internal_mouse(
    ($func:ident) => {
        Box::new(|model: &mut $crate::model::Model<'_>, _| {
            drop(model.$func());
        }) as $crate::binding::MouseAction
    };

    ($func:ident, $($arg:expr),+) => {
        Box::new(|model: &mut $crate::model::Model<'_>, _| {
            drop(model.$func($($arg),+));
        }) as $crate::binding::MouseAction
    };
);

#[macro_export]
macro_rules! do_internal_mouse_block(
    ($model:ident, $window:ident, $body:block) => {
        Box::new(|$model: &mut $crate::model::Model<'_>, $window: Option<winsys::window::Window>| {
            $body
        }) as $crate::binding::MouseAction
    };
);

#[macro_export]
macro_rules! spawn_external(
    ($cmd:expr) => {
        {
            Box::new(move |_: &mut $crate::model::Model<'_>| {
                $crate::util::Util::spawn($cmd);
            }) as $crate::binding::KeyAction
        }
    };
);

#[macro_export]
macro_rules! spawn_from_shell(
    ($cmd:expr) => {
        {
            Box::new(move |_: &mut $crate::model::Model<'_>| {
                $crate::util::Util::spawn_shell($cmd);
            }) as $crate::binding::KeyAction
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

#[macro_export]
macro_rules! build_mouse_bindings(
    { @start $mouse_bindings:expr,
        $( ( $kind:ident, $target:ident, $focus:expr ) : $binding:expr ),+ => $action:expr,
        $($tail:tt)*
    } => {
        $(
            match $crate::util::Util::parse_mouse_binding($binding) {
                None => panic!("could not parse mouse binding: {}", $binding),
                Some(shortcut) => $mouse_bindings.insert(
                    (
                        winsys::input::MouseEventKey {
                            kind: winsys::input::MouseEventKind::$kind,
                            target: winsys::input::EventTarget::$target,
                        },
                        shortcut
                    ),
                    (
                        $action,
                        $focus
                    )
                ),
            };
        )+
        build_mouse_bindings!(@start $mouse_bindings, $($tail)*);
    };

    { @start $mouse_bindings:expr,
        $($tail:tt)*
    } => {
        $(compile_error!(
            stringify!(incorrect syntax in build_mouse_bindings: $tail)
        );)*
    };

    { $($tokens:tt)+ } => {
        {
            let mut mouse_bindings = std::collections::HashMap::new();
            build_mouse_bindings!(@start mouse_bindings, $($tokens)+);
            mouse_bindings
        }
    };
);

rust_i18n::i18n!();

macro_rules! _localize_intern {
    ([$($acc:tt)*] -> [] -> []) => {$($acc)*};
    ([$($inacc:tt)*] -> [] -> [[$($outacc:tt)*] -> $rest:tt -> $ctx:tt]) => {
        _localize_intern!{[$($outacc)* {$($inacc)*}] -> $rest -> $ctx}
    };
    ([$($acc:tt)*] -> [#[doc = $docstr:tt] $($rest:tt)*] -> $ctx:tt) => {
        _localize_intern!{[$($acc)* #[command(about=rust_i18n::t!($docstr.trim()),long_about=None)]]
             -> [$($rest)*] -> $ctx}
    };
    ($acc:tt -> [{$($body:tt)*} $($rest:tt)*] -> $ctx:tt) => {
        _localize_intern!{[] -> [$($body)*] -> [$acc -> [$($rest)*] -> $ctx]}
    };
    ([$($acc:tt)*] -> [$next:tt $($rest:tt)*] -> $ctx:tt) => {
        _localize_intern!{[$($acc)* $next] -> [$($rest)*] -> $ctx}
    };
}

macro_rules! localize {
    ($($body:tt)*) => {
        _localize_intern!{[] -> [$($body)*] -> []}
    };
}

pub(crate) use _localize_intern;
pub(crate) use localize;

// Exported only for expansion in downstream crates. Each intermediate result
// is a promoted constant, so pipelines work in ordinary functions as well as
// const/static initializers, and operation arguments can use caller constants.
#[doc(hidden)]
#[macro_export]
macro_rules! __include_str_pipeline {
    ($input:expr;) => { $input };
    ($input:expr; $op:ident $(($($args:tt)*))? $(, $next:ident $(($($next_args:tt)*))?)*) => {
        $crate::__include_str_pipeline!(
            $crate::__include_str_operation!($input; $op $(($($args)*))?);
            $($next $(($($next_args)*))?),*
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __include_str_operation {
    ($input:expr; trim) => {
        $crate::__include_str_operation!(@transform $input; trim_len, trim)
    };
    ($input:expr; trim_lines) => {
        $crate::__include_str_operation!(@transform $input; trim_lines_len, trim_lines)
    };
    ($input:expr; collapse_whitespace) => {
        $crate::__include_str_operation!(@transform $input; collapse_whitespace_len, collapse_whitespace)
    };
    ($input:expr; collapse_whitespace(space $(,)?)) => {
        $crate::__include_str_operation!(@transform $input; collapse_whitespace_len, collapse_whitespace_as_space)
    };
    ($input:expr; collapse_whitespace($replacement:expr $(,)?)) => {
        $crate::__include_str_operation!(@transform $input; collapse_whitespace_with_len, collapse_whitespace_with, $replacement)
    };
    ($input:expr; sql) => {
        $crate::__include_str_operation!(@transform $input; sql_len, sql)
    };
    ($input:expr; json) => {
        $crate::__include_str_operation!(@transform $input; json_len, json)
    };
    ($input:expr; jsonc) => {
        $crate::__include_str_operation!(@transform $input; jsonc_len, jsonc)
    };
    ($input:expr; replace($from:expr, $to:expr $(,)?)) => {
        $crate::__include_str_operation!(@transform $input; replace_len, replace, $from, $to)
    };
    ($input:expr; strip_line_prefix($prefix:expr $(,)?)) => {
        $crate::__include_str_operation!(@transform $input; strip_prefix_len, strip_prefix, $prefix)
    };
    ($input:expr; strip_line_suffix($suffix:expr $(,)?)) => {
        $crate::__include_str_operation!(@transform $input; strip_suffix_len, strip_suffix, $suffix)
    };
    (@transform $input:expr; $len:ident, $transform:ident $(, $arg:expr)*) => {
        const {
            const INPUT: &str = $input;
            $crate::__private::as_str(
                &const {
                    $crate::__private::$transform::<
                        { $crate::__private::$len(INPUT $(, $arg)*) },
                    >(INPUT $(, $arg)*)
                },
            )
        }
    };
    ($input:expr; $($invalid:tt)*) => {
        ::core::compile_error!(::core::concat!(
            "include_str!: unknown operation or invalid arguments: ",
            ::core::stringify!($($invalid)*),
            "; expected trim, trim_lines, collapse_whitespace, collapse_whitespace(replacement), ",
            "replace(from, to), strip_line_prefix(prefix), strip_line_suffix(suffix), sql, json, or jsonc"
        ))
    };
}

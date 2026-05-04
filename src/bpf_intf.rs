#[allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
mod bpf_internal {
    include!(concat!(env!("OUT_DIR"), "/bpf_intf.rs"));
}

pub use bpf_internal::*;

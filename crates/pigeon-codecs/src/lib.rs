mod var_int;
mod var_long;

pub use var_int::{read_var_int, write_var_int, VarIntReadError, VarIntWriteError};
pub use var_long::{read_var_long, write_var_long, VarLongReadError, VarLongWriteError};

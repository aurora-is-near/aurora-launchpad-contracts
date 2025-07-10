module Prelude {
  datatype Option<T> = None | Some(T)
  datatype Result<T, E> = Ok(T) | Err(E)
}
module Prelude {
  datatype Option<T> = None | Some(v: T)
  datatype Result<T, E> = Ok(T) | Err(E)
}
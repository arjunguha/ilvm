use std::collections::HashMap;
use std::collections::HashSet;
use syntax;
use error::Error;
use std::hash::Hash;

fn has_unique_elements<T>(iter: T) -> bool
  where
    T: IntoIterator,
    T::Item: Eq + Hash,
{
    let mut uniq = HashSet::new();
    iter.into_iter().all(move |x| uniq.insert(x))
}

pub fn tc(blocks : Vec<syntax::Block>) ->
    Result<HashMap<i32, syntax::Instr>, Error> {
    if !has_unique_elements(blocks.iter().map(|tuple| tuple.0)) {
        return Err(Error::Usage("duplicate block IDs".to_string()));
    }

    Ok(blocks.into_iter().collect())
}

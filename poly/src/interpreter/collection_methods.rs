use super::*;

impl Interpreter {
    pub(super) fn call_list_method(&mut self, mut items: Vec<Value>, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "append" | "push" => {
                match args.get(0) {
                    Some(val) => {
                        items.push(val.clone());
                        Ok(Value::List(items))
                    }
                    None => Err(self.error("append() requires a value")),
                }
            }
            "pop" => {
                let idx = match args.get(0) {
                    Some(Value::Int(i)) => {
                        let i = if *i < 0 { (items.len() as i64 + i) as usize } else { *i as usize };
                        Some(i)
                    }
                    Some(_) => return Err(self.error("pop() index must be an integer")),
                    None => None,
                };

                if items.is_empty() {
                    return Err(self.error("pop from empty list"));
                }

                let popped = match idx {
                    Some(i) if i < items.len() => items.remove(i),
                    None => items.pop().unwrap(),
                    _ => return Err(self.error("pop index out of range")),
                };

                // Return the popped value (the list is updated via set_var in the caller)
                Ok(popped)
            }
            "insert" => {
                match (args.get(0), args.get(1)) {
                    (Some(Value::Int(idx)), Some(val)) => {
                        let idx = if *idx < 0 { (items.len() as i64 + idx) as usize } else { *idx as usize };
                        items.insert(idx.min(items.len()), val.clone());
                        Ok(Value::List(items))
                    }
                    _ => Err(self.error("insert() requires index and value")),
                }
            }
            "remove" => {
                match args.get(0) {
                    Some(val) => {
                        if let Some(pos) = items.iter().position(|v| v == val) {
                            items.remove(pos);
                        }
                        Ok(Value::List(items))
                    }
                    None => Err(self.error("remove() requires a value")),
                }
            }
            "clear" => Ok(Value::List(Vec::new())),
            "copy" => Ok(Value::List(items.clone())),
            "extend" => {
                match args.get(0) {
                    Some(Value::List(other)) => {
                        items.extend(other.clone());
                        Ok(Value::List(items))
                    }
                    _ => Err(self.error("extend() requires a list")),
                }
            }
            "index" => {
                match args.get(0) {
                    Some(val) => {
                        Ok(Value::Int(items.iter().position(|v| v == val).map(|i| i as i64).unwrap_or(-1)))
                    }
                    None => Err(self.error("index() requires a value")),
                }
            }
            "count" => {
                match args.get(0) {
                    Some(val) => {
                        Ok(Value::Int(items.iter().filter(|v| *v == val).count() as i64))
                    }
                    None => Err(self.error("count() requires a value")),
                }
            }
            "sort" => {
                items.sort_by(|a, b| self.compare_values(a, b));
                Ok(Value::List(items))
            }
            "reverse" => {
                items.reverse();
                Ok(Value::List(items))
            }
            "join" => {
                match args.get(0) {
                    Some(Value::String(sep)) => {
                        let strings: Vec<String> = items.iter().map(|v| format!("{}", v)).collect();
                        Ok(Value::String(strings.join(sep)))
                    }
                    _ => Err(self.error("join() requires a separator string")),
                }
            }
            _ => Err(self.error(format!("Unknown list method: {}", method))),
        }
    }

    /// Call a dict method
    pub(super) fn call_dict_method(&mut self, pairs: Vec<(Value, Value)>, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "keys" => {
                Ok(Value::List(pairs.iter().map(|(k, _)| k.clone()).collect()))
            }
            "values" => {
                Ok(Value::List(pairs.iter().map(|(_, v)| v.clone()).collect()))
            }
            "items" => {
                Ok(Value::List(pairs.iter().map(|(k, v)| Value::List(vec![k.clone(), v.clone()])).collect()))
            }
            "get" => {
                let key = args.get(0).ok_or_else(|| self.error("get() requires a key"))?;
                let default = args.get(1).cloned().unwrap_or(Value::None);

                for (k, v) in &pairs {
                    if k == key {
                        return Ok(v.clone());
                    }
                }
                Ok(default)
            }
            "pop" => {
                let key = args.get(0).ok_or_else(|| self.error("pop() requires a key"))?;
                let default = args.get(1).cloned();

                for (k, v) in &pairs {
                    if k == key {
                        return Ok(v.clone());
                    }
                }

                match default {
                    Some(d) => Ok(d),
                    None => Err(self.error("Key not found")),
                }
            }
            "update" => {
                match args.get(0) {
                    Some(Value::Dict(other)) => {
                        let mut new_pairs = pairs.clone();
                        for (k, v) in other {
                            let mut found = false;
                            for (ek, ev) in new_pairs.iter_mut() {
                                if ek == k {
                                    *ev = v.clone();
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                new_pairs.push((k.clone(), v.clone()));
                            }
                        }
                        Ok(Value::Dict(new_pairs))
                    }
                    _ => Err(self.error("update() requires a dict")),
                }
            }
            "clear" => Ok(Value::Dict(Vec::new())),
            "copy" => Ok(Value::Dict(pairs.clone())),
            _ => Err(self.error(format!("Unknown dict method: {}", method))),
        }
    }
}

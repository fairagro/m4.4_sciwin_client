use commonwl::{DefaultValue, ScatterMethod};
use std::{collections::HashMap, error::Error};

use crate::InputObject;

pub fn gather_jobs(
    scatter_inputs: &[Vec<DefaultValue>],
    scatter_keys: &[String],
    method: &ScatterMethod,
) -> Result<Vec<HashMap<String, DefaultValue>>, Box<dyn Error>> {
    match method {
        ScatterMethod::DotProduct => {
            let len = scatter_inputs[0].len();
            if scatter_inputs.iter().any(|arr| arr.len() != len) {
                return Err("All scatter inputs must be the same length for dotproduct.".into());
            }

            let jobs = (0..len)
                .map(|i| {
                    scatter_keys
                        .iter()
                        .cloned()
                        .zip(scatter_inputs.iter().map(|arr| arr[i].clone()))
                        .collect::<HashMap<_, _>>()
                })
                .collect::<Vec<_>>();
            Ok(jobs)
        }
        // a little Chad Gippity was used to get what the Docu was sayin' about the Flat CP
        ScatterMethod::FlatCrossProduct => {
            let mut jobs = vec![HashMap::new()];
            for (key, values) in scatter_keys.iter().zip(scatter_inputs.iter()) {
                let mut new_jobs = Vec::new();
                for job in &jobs {
                    for value in values {
                        let mut new_job = job.clone();
                        new_job.insert(key.clone(), value.clone());
                        new_jobs.push(new_job);
                    }
                }
                jobs = new_jobs;
            }
            Ok(jobs)
        }
        ScatterMethod::NestedCrossProduct => {
            fn nest(
                keys: &[String],
                values: &[Vec<DefaultValue>],
                index: usize,
                current: &mut HashMap<String, DefaultValue>,
                jobs: &mut Vec<HashMap<String, DefaultValue>>,
            ) {
                if index == keys.len() {
                    jobs.push(current.clone());
                } else {
                    for v in &values[index] {
                        current.insert(keys[index].clone(), v.clone());
                        nest(keys, values, index + 1, current, jobs);
                    }
                }
            }

            let mut jobs = vec![];
            let mut current = HashMap::new();
            nest(scatter_keys, scatter_inputs, 0, &mut current, &mut jobs);
            Ok(jobs)
        }
    }
}

pub fn gather_inputs(scatter_keys: &[String], input_values: &InputObject) -> Result<Vec<Vec<DefaultValue>>, Box<dyn Error>> {
    scatter_keys
        .iter()
        .map(|k| {
            input_values
                .inputs
                .get(k)
                .and_then(|v| match v {
                    DefaultValue::Array(arr) => Some(arr.clone()),
                    _ => None,
                })
                .ok_or_else(|| Box::from(format!("Input {k} must be of type array to scatter!")))
        })
        .collect::<Result<Vec<Vec<DefaultValue>>, _>>()
}

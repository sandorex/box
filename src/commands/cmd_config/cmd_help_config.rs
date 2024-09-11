use documented::DocumentedFields;
use struct_field_names_as_array::FieldNamesAsArray;
use crate::config::Config;
use crate::ExitResult;

pub fn help_config() -> ExitResult {
    for (i, name) in Config::FIELD_NAMES_AS_ARRAY.iter().enumerate() {
        println!("{}", Config::FIELD_DOCS[i].unwrap_or("??"));
        println!("{name}\n");
    }

    Ok(())
}

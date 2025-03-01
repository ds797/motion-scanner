use crate::wl_types::ArgType;

pub fn type_from_string(typ: String) -> ArgType {
	match typ.as_str() {
		"int" => Some(ArgType::Int),
		"uint" => Some(ArgType::Uint),
		"fixed" => Some(ArgType::Fixed),
		"string" => Some(ArgType::String),
		"object" => Some(ArgType::ObjectId),
		"new_id" => Some(ArgType::NewId),
		"array" => Some(ArgType::Array),
		"fd" => Some(ArgType::Fd),
		_ => None
	}.expect("Unknown type")
}
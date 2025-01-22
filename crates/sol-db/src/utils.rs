use prettytable::{Cell, Row, Table};
use serde_json::Value;

/// Print a collection of JSON objects as an ASCII table
pub fn print_json_objects_as_table(json_objects: &Vec<Value>) {
    // Create a new table
    let mut table = Table::new();

    // Check if there are objects to print
    if json_objects.is_empty() {
        println!("No data to display.");
        return;
    }

    // Get the keys from the first object for headers
    if let Some(Value::Object(first_obj)) = json_objects.first() {
        let headers: Vec<&str> = first_obj.keys().map(|k| k.as_str()).collect();
        table.add_row(Row::new(
            headers.iter().map(|&header| Cell::new(header)).collect(),
        ));

        // Add rows for each JSON object
        for json_obj in json_objects {
            if let Value::Object(obj) = json_obj {
                let row_values: Vec<String> = headers
                    .iter()
                    .map(|key| {
                        obj.get(*key)
                            .map(|v| v.to_string()) // Convert JSON value to string
                            .unwrap_or_else(|| "NULL".to_string())
                    })
                    .collect();
                table.add_row(Row::new(
                    row_values.iter().map(|value| Cell::new(value)).collect(),
                ));
            }
        }
    } else {
        println!("Invalid JSON structure. Expected a list of JSON objects.");
        return;
    }

    // Print the table
    table.printstd();
}

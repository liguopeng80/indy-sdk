extern crate libc;
extern crate serde_json;

use self::libc::c_char;
use utils::cstring::CStringUtils;
use utils::error;
use utils::error::error_string;
use std::ptr;
use schema;
use settings;
use error::ToErrorCode;
use utils::threadpool::spawn;

/// Create a new Schema object that can create or look up schemas on the ledger
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// source_id: Enterprise's personal identification for the user.
///
/// schema_name: Name of schema
///
/// version: version of schema
///
/// schema_data: list of attributes that will make up the schema
///
/// # Example schema_data -> "["attr1", "attr2", "attr3"]"
///
/// payment_handle: future use (currently uses any address in the wallet)
///
/// cb: Callback that provides Schema handle and error status of request.
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern fn vcx_schema_create(command_handle: u32,
                                source_id: *const c_char,
                                schema_name: *const c_char,
                                version: *const c_char,
                                schema_data: *const c_char,
                                payment_handle: u32,
                                cb: Option<extern fn(xcommand_handle: u32, err: u32, credentialdef_handle: u32)>) -> u32 {
    check_useful_c_callback!(cb, error::INVALID_OPTION.code_num);
    check_useful_c_str!(schema_name, error::INVALID_OPTION.code_num);
    check_useful_c_str!(version, error::INVALID_OPTION.code_num);
    check_useful_c_str!(source_id, error::INVALID_OPTION.code_num);
    check_useful_c_str!(schema_data, error::INVALID_OPTION.code_num);

    let issuer_did = match settings::get_config_value(settings::CONFIG_INSTITUTION_DID) {
        Ok(x) => x,
        Err(x) => return x
    };
    info!(target:"vcx","vcx_schema_create(command_handle: {}, source_id: {}, schema_name: {},  schema_data: {})",
          command_handle, source_id, schema_name, schema_data);

    spawn(move|| {
        let ( rc, handle) = match schema::create_new_schema(&source_id,
                                                            issuer_did,
                                                            schema_name,
                                                            version,
                                                            schema_data) {
            Ok(x) => {
                info!(target:"vcx", "vcx_schema_create_cb(command_handle: {}, rc: {}, handle: {}) source_id: {}",
                      command_handle, error_string(0), x, source_id);
                (error::SUCCESS.code_num, x)
            },
            Err(x) => {
                warn!("vcx_schema_create_cb(command_handle: {}, rc: {}, handle: {}) source_id: {}",
                      command_handle, error_string(x.to_error_code()), 0, source_id);
                (x.to_error_code(), 0) },
        };

        cb(command_handle, rc, handle);

        Ok(())
    });

    error::SUCCESS.code_num
}


/// Takes the schema object and returns a json string of all its attributes
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// schema_handle: Schema handle that was provided during creation. Used to access schema object
///
/// cb: Callback that provides json string of the schema's attributes and provides error status
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern fn vcx_schema_serialize(command_handle: u32,
                                   schema_handle: u32,
                                   cb: Option<extern fn(xcommand_handle: u32, err: u32, schema_state: *const c_char)>) -> u32 {

    check_useful_c_callback!(cb, error::INVALID_OPTION.code_num);

    let source_id = schema::get_source_id(schema_handle).unwrap_or_default();
    info!("vcx_schema_serialize(command_handle: {}, schema_handle: {}) source_id: {}",
          command_handle, schema_handle, source_id);

    if !schema::is_valid_handle(schema_handle) {
        return error::INVALID_SCHEMA_HANDLE.code_num;
    };

    spawn(move|| {
        match schema::to_string(schema_handle) {
            Ok(x) => {
                info!("vcx_schema_serialize_cb(command_handle: {}, schema_handle: {}, rc: {}, state: {}) source_id: {}",
                      command_handle, schema_handle, error_string(0), x, source_id);
                let msg = CStringUtils::string_to_cstring(x);
                cb(command_handle, error::SUCCESS.code_num, msg.as_ptr());
            },
            Err(x) => {
                warn!("vcx_schema_serialize_cb(command_handle: {}, schema_handle: {}, rc: {}, state: {}) source_id: {}",
                      command_handle, schema_handle, error_string(x.to_error_code()), "null", source_id);
                cb(command_handle, x.to_error_code(), ptr::null_mut());
            },
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Takes a json string representing a schema object and recreates an object matching the json
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// schema_data: json string representing a schema object
///
/// cb: Callback that provides schema handle and provides error status
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern fn vcx_schema_deserialize(command_handle: u32,
                                     schema_data: *const c_char,
                                     cb: Option<extern fn(xcommand_handle: u32, err: u32, schema_handle: u32)>) -> u32 {

    check_useful_c_callback!(cb, error::INVALID_OPTION.code_num);
    check_useful_c_str!(schema_data, error::INVALID_OPTION.code_num);

    info!("vcx_schema_deserialize(command_handle: {}, schema_data: {})", command_handle, schema_data);
    spawn(move|| {
        let (rc, handle) = match schema::from_string(&schema_data) {
            Ok(x) => {
                info!("vcx_schema_deserialize_cb(command_handle: {}, rc: {}, handle: {}), source_id: {}",
                      command_handle, error_string(0), x, schema::get_source_id(x).unwrap_or_default());
                (error::SUCCESS.code_num, x)
            },
            Err(x) => {
                warn!("vcx_schema_deserialize_cb(command_handle: {}, rc: {}, handle: {}), source_id: {}",
                      command_handle, error_string(x.to_error_code()), 0, "");
                (x.to_error_code(), 0)
            },
        };
        cb(command_handle, rc, handle);

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Releases the schema object by de-allocating memory
///
/// #Params
/// schema_handle: Schema handle that was provided during creation. Used to access schema object
///
/// #Returns
/// Success
#[no_mangle]
pub extern fn vcx_schema_release(schema_handle: u32) -> u32 {
    let source_id = schema::get_source_id(schema_handle).unwrap_or_default();
    match schema::release(schema_handle) {
        Ok(x) => info!("vcx_schema_release(schema_handle: {}, rc: {}), source_id: {}",
                       schema_handle, error_string(0), source_id),
        Err(e) => warn!("vcx_schema_release(schema_handle: {}, rc: {}), source_id: {}",
                       schema_handle, error_string(e.to_error_code()), source_id),
    };
    error::SUCCESS.code_num
}

/// Retrieves schema's id
///
/// #Params
/// schema_handle: Schema handle that was provided during creation. Used to access proof object
///
/// cb: Callback that provides schema id and provides error status
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern fn vcx_schema_get_schema_id(command_handle: u32,
                                       schema_handle: u32,
                                       cb: Option<extern fn(xcommand_handle: u32, err: u32, schema_id: *const c_char)>) -> u32 {
    check_useful_c_callback!(cb, error::INVALID_OPTION.code_num);

    info!("vcx_schema_get_schema_id(command_handle: {}, schema_handle: {})", command_handle, schema_handle);
    if !schema::is_valid_handle(schema_handle) {
        return error::INVALID_SCHEMA_HANDLE.code_num;
    }

    spawn(move|| {
        match schema::get_schema_id(schema_handle) {
            Ok(x) => {
                info!("vcx_schema_get_schema_id(command_handle: {}, schema_handle: {}, rc: {}, schema_seq_no: {})",
                      command_handle, schema_handle, error_string(0), x);
                let msg = CStringUtils::string_to_cstring(x);
                cb(command_handle, error::SUCCESS.code_num, msg.as_ptr());
            },
            Err(x) => {
                warn!("vcx_schema_get_schema_id(command_handle: {}, schema_handle: {}, rc: {}, schema_seq_no: {})",
                      command_handle, schema_handle, x.to_string(), "");
                cb(command_handle, x.to_error_code(), ptr::null_mut());
            },
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Retrieves all of the data associated with a schema on the ledger.
///
/// #Params
/// source_id: Enterprise's personal identification for the user.
///
/// schema_id: id of schema given during the creation of the schema
///
/// cb: Callback contains the error status (if the schema cannot be found)
/// and it will also contain a json string representing all of the data of a
/// schema already on the ledger.
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern fn vcx_schema_get_attributes(command_handle: u32,
                                        source_id: *const c_char,
                                        schema_id: *const c_char,
                                        cb: Option<extern fn(xcommand_handle: u32, err: u32, s_handle: u32, schema_attrs: *const c_char)>) -> u32 {
    check_useful_c_callback!(cb, error::INVALID_OPTION.code_num);
    check_useful_c_str!(source_id, error::INVALID_OPTION.code_num);
    check_useful_c_str!(schema_id, error::INVALID_OPTION.code_num);
    info!("vcx_schema_get_attributes(command_handle: {}, source_id: {}, schema_id: {})",
          command_handle, source_id, schema_id);

    spawn(move|| {
        match schema::get_schema_attrs(source_id, schema_id) {
            Ok((handle, data)) => {
                let data:serde_json::Value = serde_json::from_str(&data).unwrap();
                let data = data["data"].clone();
                info!("vcx_schema_get_attributes_cb(command_handle: {}, rc: {}, handle: {}, attrs: {})",
                      command_handle, error_string(0), handle, data);
                let msg = CStringUtils::string_to_cstring(data.to_string());
                cb(command_handle, error::SUCCESS.code_num, handle, msg.as_ptr());
            },
            Err(x) => {
                warn!("vcx_schema_get_attributes_cb(command_handle: {}, rc: {}, handle: {}, attrs: {})",
                      command_handle, error_string(x.to_error_code()), 0, "");
                cb(command_handle, x.to_error_code(), 0, ptr::null_mut());
            },
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Retrieve the txn associated with paying for the schema
///
/// #param
/// handle: schema handle that was provided during creation.  Used to access schema object.
///
/// #Callback returns
/// PaymentTxn json
/// example: {
///         "amount":25,
///         "inputs":[
///             "pay:null:1_3FvPC7dzFbQKzfG",
///             "pay:null:1_lWVGKc07Pyc40m6"
///         ],
///         "outputs":[
///             {"recipient":"pay:null:FrSVC3IrirScyRh","amount":5,"extra":null},
///             {"recipient":"pov:null:OsdjtGKavZDBuG2xFw2QunVwwGs5IB3j","amount":25,"extra":null}
///         ]
///     }
#[no_mangle]
pub extern fn vcx_schema_get_payment_txn(command_handle: u32,
                                             handle: u32,
                                             cb: Option<extern fn(xcommand_handle: u32, err: u32, txn: *const c_char)>) -> u32 {

    check_useful_c_callback!(cb, error::INVALID_OPTION.code_num);

    info!("vcx_schema_get_payment_txn(command_handle: {})", command_handle);

    spawn(move|| {
        match schema::get_payment_txn(handle) {
            Ok(x) => {
                match serde_json::to_string(&x) {
                    Ok(x) => {
                        info!("vcx_schema_get_payment_txn_cb(command_handle: {}, rc: {}, : {}), source_id: {:?}",
                              command_handle, error_string(0), x, schema::get_source_id(handle).unwrap_or_default());

                        let msg = CStringUtils::string_to_cstring(x);
                        cb(command_handle, 0, msg.as_ptr());
                    }
                    Err(_) => {
                        error!("vcx_schema_get_payment_txn_cb(command_handle: {}, rc: {}, txn: {}), source_id: {:?}",
                               command_handle, error_string(error::INVALID_JSON.code_num), "null", schema::get_source_id(handle).unwrap_or_default());
                        cb(command_handle, error::INVALID_JSON.code_num, ptr::null_mut());
                    }
                }
            },
            Err(x) => {
                error!("vcx_schema_get_payment_txn_cb(command_handle: {}, rc: {}, txn: {}), source_id: {:?}",
                       command_handle, x.to_string(), "null", schema::get_source_id(handle).unwrap_or_default());
                cb(command_handle, x.to_error_code(), ptr::null());
            },
        };

        Ok(())
    });

    error::SUCCESS.code_num
}


#[cfg(test)]
mod tests {
    extern crate serde_json;
    extern crate rand;

    use super::*;
    #[allow(unused_imports)]
    use rand::Rng;
    use std::ffi::CString;
    use std::time::Duration;
    use settings;
    #[allow(unused_imports)]
    use utils::constants::{ SCHEMA_ID, SCHEMA_WITH_VERSION, DEFAULT_SCHEMA_ATTRS, DEFAULT_SCHEMA_ID, DEFAULT_SCHEMA_NAME };
    use api::return_types_u32;

    #[test]
    fn test_vcx_create_schema_success() {
        init!("true");
        let cb = return_types_u32::Return_U32_U32::new().unwrap();
        assert_eq!(vcx_schema_create(cb.command_handle,
                                       CString::new("Test Source ID").unwrap().into_raw(),
                                       CString::new("Test Schema").unwrap().into_raw(),
                                       CString::new("0.0").unwrap().into_raw(),
                                       CString::new("[att1, att2]").unwrap().into_raw(),
                                       0,
                                       Some(cb.get_callback())), error::SUCCESS.code_num);
        let handle = cb.receive(Some(Duration::from_secs(2))).unwrap();
        assert!(handle>0)
    }

    #[cfg(feature="pool_tests")]
    #[test]
    fn test_vcx_create_schema_with_pool() {
        init!("ledger");

        let data = r#"["name","male"]"#;
        let schema_name: String = rand::thread_rng().gen_ascii_chars().take(25).collect::<String>();
        let schema_version: String = format!("{}.{}",rand::thread_rng().gen::<u32>().to_string(),
                                             rand::thread_rng().gen::<u32>().to_string());

        let cb = return_types_u32::Return_U32_U32::new().unwrap();
        assert_eq!(vcx_schema_create(cb.command_handle,
                                     CString::new("Test Source ID").unwrap().into_raw(),
                                     CString::new(schema_name).unwrap().into_raw(),
                                     CString::new(schema_version).unwrap().into_raw(),
                                     CString::new(data).unwrap().into_raw(),
                                     0,
                                     Some(cb.get_callback())), error::SUCCESS.code_num);

        let handle = cb.receive(Some(Duration::from_secs(5))).unwrap();
    }

    #[cfg(feature="pool_tests")]
    #[test]
    fn test_vcx_schema_get_attrs_with_pool() {
        init!("ledger");
        let (schema_id, _) = ::utils::libindy::anoncreds::tests::create_and_write_test_schema(::utils::constants::DEFAULT_SCHEMA_ATTRS);

        let cb = return_types_u32::Return_U32_U32_STR::new().unwrap();
        assert_eq!(vcx_schema_get_attributes(cb.command_handle,
                                     CString::new("Test Source ID").unwrap().into_raw(),
                                     CString::new(schema_id).unwrap().into_raw(),
                                     Some(cb.get_callback())), error::SUCCESS.code_num);

        let (err, attrs) = cb.receive(Some(Duration::from_secs(2))).unwrap();
        let mut result_vec = vec!(attrs.clone().unwrap());
        let mut expected_vec = vec!(DEFAULT_SCHEMA_ATTRS);
        assert_eq!(result_vec.sort(), expected_vec.sort());
    }

    #[test]
    fn test_vcx_schema_serialize() {
        init!("true");
        let data = r#"["name","male"]"#;
        let cb = return_types_u32::Return_U32_U32::new().unwrap();
        assert_eq!(vcx_schema_create(cb.command_handle,
                                     CString::new("Test Source ID").unwrap().into_raw(),
                                     CString::new("Test Schema").unwrap().into_raw(),
                                     CString::new("0.0.0").unwrap().into_raw(),
                                     CString::new(data).unwrap().into_raw(),
                                     0,
                                     Some(cb.get_callback())), error::SUCCESS.code_num);
        let handle = cb.receive(Some(Duration::from_millis(200))).unwrap();
        assert!(handle > 0);
    }

    #[test]
    fn test_vcx_schema_deserialize_succeeds() {
        init!("true");
        let cb = return_types_u32::Return_U32_U32::new().unwrap();
        let err = vcx_schema_deserialize(cb.command_handle,CString::new(SCHEMA_WITH_VERSION).unwrap().into_raw(), Some(cb.get_callback()));
        assert_eq!(err, error::SUCCESS.code_num);
        let schema_handle = cb.receive(Some(Duration::from_secs(2))).unwrap();
        assert!(schema_handle > 0);
    }

    #[test]
    fn test_vcx_schema_get_schema_id_succeeds() {
        init!("true");
        let data = r#"["name","male"]"#;
        let cb = return_types_u32::Return_U32_U32::new().unwrap();
        assert_eq!(vcx_schema_create(cb.command_handle,
                                     CString::new("Test Source ID").unwrap().into_raw(),
                                     CString::new(DEFAULT_SCHEMA_NAME).unwrap().into_raw(),
                                     CString::new("0.0.0").unwrap().into_raw(),
                                     CString::new(data).unwrap().into_raw(),
                                     0,
                                     Some(cb.get_callback())), error::SUCCESS.code_num);
        let schema_handle = cb.receive(Some(Duration::from_secs(2))).unwrap();
        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        assert_eq!(vcx_schema_get_schema_id(cb.command_handle, schema_handle, Some(cb.get_callback())), error::SUCCESS.code_num);
        let id = cb.receive(Some(Duration::from_secs(2))).unwrap().unwrap();
        assert_eq!(DEFAULT_SCHEMA_ID, &id);

    }

    #[test]
    fn test_vcx_schema_get_attrs() {
        init!("true");
        settings::set_config_value(settings::CONFIG_ENABLE_TEST_MODE, "true");
        let cb = return_types_u32::Return_U32_U32_STR::new().unwrap();
        let data = r#"["height","name","sex","age"]"#;
        assert_eq!(vcx_schema_get_attributes(cb.command_handle,
                                             CString::new("Test Source ID").unwrap().into_raw(),
                                             CString::new(SCHEMA_ID).unwrap().into_raw(),
                                             Some(cb.get_callback())), error::SUCCESS.code_num);
        let (handle, schema_data_as_string) = cb.receive(Some(Duration::from_secs(2))).unwrap();
        let schema_data_as_string = schema_data_as_string.unwrap();
        let schema_as_json:serde_json::Value = serde_json::from_str(&schema_data_as_string).unwrap();
        assert_eq!(schema_as_json["data"].to_string(), data);
    }

    #[test]
    fn test_get_payment_txn() {
        init!("true");
        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        let did = settings::get_config_value(settings::CONFIG_INSTITUTION_DID).unwrap();
        let handle = schema::create_new_schema("testid", did, "name".to_string(),"1.0".to_string(),"[\"name\":\"male\"]".to_string()).unwrap();
        let rc = vcx_schema_get_payment_txn(cb.command_handle, handle, Some(cb.get_callback()));
        let txn = cb.receive(Some(Duration::from_secs(2))).unwrap();
        assert!(txn.is_some());
    }

    #[cfg(feature = "pool_tests")]
    #[test]
    fn test_vcx_schema_serialize_contains_version() {
        init!("ledger");
        let cb = return_types_u32::Return_U32_U32::new().unwrap();
        let schema_name= format!("TestSchema-{}", rand::thread_rng().gen::<u32>());
        let source_id = "Test Source ID";
        assert_eq!(vcx_schema_create(cb.command_handle,
                                     CString::new(source_id).unwrap().into_raw(),
                                     CString::new(schema_name).unwrap().into_raw(),
                                     CString::new("0.0.0").unwrap().into_raw(),
                                     CString::new(r#"["name","dob"]"#).unwrap().into_raw(),
                                     0,
                                     Some(cb.get_callback())), error::SUCCESS.code_num);
        let handle = match cb.receive(Some(Duration::from_secs(5))) {
            Ok(h) => h,
            Err(e) => panic!("Error Creating serialized schema: {}", e),
        };

        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        assert_eq!(vcx_schema_serialize(cb.command_handle, handle, Some(cb.get_callback())), error::SUCCESS.code_num);
        let data = cb.receive(Some(Duration::from_secs(2))).unwrap().unwrap();
        use schema::CreateSchema;
        let j:serde_json::Value = serde_json::from_str(&data.clone()).unwrap();
        let schema:CreateSchema = serde_json::from_value(j["data"].clone()).unwrap();
        assert_eq!(j["version"], "1.0");
        assert_eq!(schema.get_source_id(), source_id);
    }
}

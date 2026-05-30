//! RPC abstraction: the `RpcCaller` trait, the live `ZoteroRpc` implementation,
//! and small client-call helpers.

use serde_json::Value;
use zotron_rpc::ZoteroRpc;

pub trait RpcCaller {
    fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value, String>;
}

impl RpcCaller for ZoteroRpc {
    fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value, String> {
        self.call(method, params).map_err(|err| err.to_string())
    }
}

pub(crate) fn call_json(
    client: &mut impl RpcCaller,
    method: &str,
    params: Option<Value>,
) -> Result<Value, String> {
    client.call(method, params)
}

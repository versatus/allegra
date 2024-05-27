use crate::{
    params::Payload,
    allegra_rpc::{
        InstanceCreateParams,
        InstanceStopParams,
        InstanceDeleteParams,
        InstanceExposeServiceParams,
        InstanceGetSshDetails,
        InstanceAddPubkeyParams,
        InstanceStartParams
    }
};


impl Payload for InstanceCreateParams {
    fn into_payload(&self) -> String {
       todo!() 
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        todo!()
    }
}

impl Payload for InstanceStartParams {
    fn into_payload(&self) -> String {
       todo!() 
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        todo!()
    }
}

impl Payload for InstanceStopParams {
    fn into_payload(&self) -> String {
       todo!() 
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        todo!()
    }
}

impl Payload for InstanceDeleteParams {
    fn into_payload(&self) -> String {
       todo!() 
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        todo!()
    }
}

impl Payload for InstanceExposeServiceParams {
    fn into_payload(&self) -> String {
       todo!() 
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        todo!()
    }
}

impl Payload for InstanceGetSshDetails {
    fn into_payload(&self) -> String {
       todo!() 
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        todo!()
    }
}

impl Payload for InstanceAddPubkeyParams {
    fn into_payload(&self) -> String {
       todo!() 
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        todo!()
    }
}

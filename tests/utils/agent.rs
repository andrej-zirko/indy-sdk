use std::sync::mpsc::{channel};
use std::ffi::{CString};

use sovrin::api::agent::{
    sovrin_agent_close_connection,
    sovrin_agent_close_listener,
    sovrin_agent_connect,
    sovrin_agent_listen,
    sovrin_agent_send,
};
use sovrin::api::ErrorCode;

use utils::callback::CallbackUtils;
use utils::timeout::TimeoutUtils;

pub struct AgentUtils {}

impl AgentUtils {
    pub fn connect(wallet_handle: i32, sender_did: &str, receiver_did: &str,
                   on_msg: Option<Box<Fn(i32, String) + Send>>) -> Result<i32, ErrorCode> {
        let (sender, receiver) = channel();
        let closure = Box::new(move |err, connection_handle| { sender.send((err, connection_handle)).unwrap(); });
        let (cmd_connect, cb) = CallbackUtils::closure_to_agent_connect_cb(closure);
        let (cb_id, msg_cb) = CallbackUtils::closure_to_agent_message_cb(Box::new(move |conn_handle, err, msg| {
            info!("On connection {} received (with error {:?}) agent message (SRV->CLI): {}", conn_handle, err, msg);
            if let Some(ref on_msg) = on_msg {
                on_msg(conn_handle, msg);
            }
        })); //TODO make as parameter?

        let err = sovrin_agent_connect(cmd_connect, wallet_handle,
                                       CString::new(sender_did).unwrap().as_ptr(),
                                       CString::new(receiver_did).unwrap().as_ptr(),
                                       cb, msg_cb);
        if err != ErrorCode::Success {
            return Err(err);
        }

        let (err, conn_handle) = receiver.recv_timeout(TimeoutUtils::medium_timeout()).unwrap();
        if err != ErrorCode::Success {
            return Err(err);
        }
        CallbackUtils::closure_map_ids(cb_id, conn_handle);

        Ok(conn_handle)
    }

    pub fn listen(wallet_handle: i32, endpoint: &str,
                  on_connect: Option<Box<Fn(i32, i32) + Send>>,
                  on_msg: Option<Box<Fn(i32, String) + Send>>) -> Result<i32, ErrorCode> {
        let (sender, receiver) = channel();
        let on_msg = Box::new(move |conn_handle, err, msg| {
            info!("On connection {} received (with error {:?}) agent message (CLI->SRV): {}", conn_handle, err, msg);
            if let Some(ref on_msg) = on_msg {
                on_msg(conn_handle, msg);
            }
        });
        let (on_msg_cb_id, on_msg) = CallbackUtils::closure_to_agent_message_cb(on_msg);

        let on_connect = Box::new(move |listener_handle, err, conn_handle, sender_did, receiver_did| {
            if let Some(ref on_connect) = on_connect {
                on_connect(listener_handle, conn_handle);
            }
            CallbackUtils::closure_map_ids(on_msg_cb_id, conn_handle);
            info!("New connection {} on listener {}, err {:?}, sender DID {}, receiver DID {}", conn_handle, listener_handle, err, sender_did, receiver_did);
        });
        let (on_connect_cb_id, on_connect) = CallbackUtils::closure_to_agent_connected_cb(on_connect);

        let cb = Box::new(move |err, listener_handle| sender.send((err, listener_handle)).unwrap());
        let (cmd_id, cb) = CallbackUtils::closure_to_agent_listen_cb(cb);

        let res = sovrin_agent_listen(cmd_id, wallet_handle, CString::new(endpoint).unwrap().as_ptr(), cb, on_connect, on_msg);

        if res != ErrorCode::Success {
            return Err(res);
        }

        let (res, listener_handle) = receiver.recv_timeout(TimeoutUtils::short_timeout()).unwrap();
        CallbackUtils::closure_map_ids(on_connect_cb_id, listener_handle);
        if res != ErrorCode::Success {
            return Err(res);
        }

        Ok(listener_handle)
    }

    pub fn send(conn_handle: i32, msg: &str) -> Result<(), ErrorCode> {
        let (send_sender, send_receiver) = channel();
        let (send_cmd_id, send_cb) = CallbackUtils::closure_to_agent_send_cb(
            Box::new(move |err_code| send_sender.send(err_code).unwrap())
        );

        let res = sovrin_agent_send(send_cmd_id, conn_handle, CString::new(msg).unwrap().as_ptr(), send_cb);
        if res != ErrorCode::Success {
            return Err(res);
        }

        let res = send_receiver.recv_timeout(TimeoutUtils::short_timeout()).unwrap();
        if res != ErrorCode::Success {
            return Err(res)
        }

        Ok(())
    }

    pub fn close_connection(conn_handle: i32) -> Result<(), ErrorCode> {
        let (sender, receiver) = channel();
        let (cmd_id, cb) = CallbackUtils::closure_to_agent_close_cb(Box::new(move |res| {
            sender.send(res).unwrap();
        }));

        let res = sovrin_agent_close_connection(cmd_id, conn_handle, cb);
        if res != ErrorCode::Success {
            return Err(res);
        }

        let res = receiver.recv_timeout(TimeoutUtils::medium_timeout()).unwrap();
        if res != ErrorCode::Success {
            return Err(res);
        }

        Ok(())
    }

    pub fn close_listener(listener_handle: i32) -> Result<(), ErrorCode> {
        let (sender, receiver) = channel();
        let (cmd_id, cb) = CallbackUtils::closure_to_agent_close_cb(Box::new(move |res| {
            sender.send(res).unwrap();
        }));

        let res = sovrin_agent_close_listener(cmd_id, listener_handle, cb);
        if res != ErrorCode::Success {
            return Err(res);
        }

        let res = receiver.recv_timeout(TimeoutUtils::medium_timeout()).unwrap();
        if res != ErrorCode::Success {
            return Err(res);
        }

        Ok(())
    }
}
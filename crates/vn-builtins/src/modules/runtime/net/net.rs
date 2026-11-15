pub mod driver;
mod http_head;
pub mod http_parser;

use driver::driver;
use urlencoding::{decode, encode};
use vn_core::{ErrorField, StdErrorClass};
use vn_macros::natives;
use vn_types::NativeError;
use vn_types::{HostOpen, NativeCtx, VmValue};


fn payload(ctx: &mut dyn NativeCtx, data: VmValue) -> Result<Vec<u8>, String> {
    if ctx.is_buffer(data) {
        ctx.buffer_to_bytes(data)
            .ok_or_else(|| "net: could not read the Bytes argument".to_string())
    } else if let Some(text) = ctx.str_owned(data) {
        Ok(text.into_bytes())
    } else {
        Err("net: expected Bytes or str".to_string())
    }
}

natives! {
    isIP(s) => s.parse::<std::net::IpAddr>().is_ok(),
    isIPv4(s) => s.parse::<std::net::Ipv4Addr>().is_ok(),
    isIPv6(s) => s.parse::<std::net::Ipv6Addr>().is_ok(),
    encodeURIComponent(value) => encode(value).into_owned(),
    decodeURIComponent(value) => {
        (decode(value)
            .map(|d| d.into_owned())
            .map_err(|e| format!("Net.decodeURIComponent: {e}"))).map_err(|e| NativeError::std(StdErrorClass::Parse, e))
    },
    tcpListen(ctx, port, host) => {
        if !ctx.check_net_listen(port) {
            return Err(NativeError::capability("net.server", port.to_string()));
        }
        driver()
            .listen(port, host)
            .map_err(|e| NativeError::std(StdErrorClass::Net, e.to_string()).with_str(ErrorField::Host, host).with_int(ErrorField::Port, port))
    },
    tcpAccept(ctx, listener_id) => {
        let task = driver().accept(listener_id);
        Ok(ctx.task_from_host(task, HostOpen::Plain))
    },
    tcpConnect(ctx, host, port) => {
        if !ctx.check_net_connect(host) {
            return Err(NativeError::capability("net.client", host));
        }
        let task = driver().connect(host, port);
        Ok(ctx.task_from_host(task, HostOpen::Plain))
    },
    tcpConnectTls(ctx, host, port, ca, identity) => {
        if !ctx.check_net_connect(host) {
            return Err(NativeError::capability("net.client", host));
        }
        let task = driver().connect_tls(host, port, ca, identity);
        Ok(ctx.task_from_host(task, HostOpen::Plain))
    },
    tcpListenTls(ctx, port, host, cert, key, client_ca) => {
        if !ctx.check_net_listen(port) {
            return Err(NativeError::capability("net.server", port.to_string()));
        }
        driver()
            .listen_tls(port, host, cert, key, client_ca)
            .map_err(|e| NativeError::std(StdErrorClass::Net, e).with_str(ErrorField::Host, host).with_int(ErrorField::Port, port))
    },
    tcpRead(ctx, conn_id, len) => {
        let task = driver().read(conn_id, len.max(0) as usize);
        Ok(ctx.task_from_host(task, HostOpen::Plain))
    },
    tcpWrite(ctx, conn_id, data) => {
        let bytes = payload(ctx, data).map_err(|e| NativeError::std(StdErrorClass::Net, e))?;
        let task = driver().write(conn_id, bytes);
        Ok(ctx.task_from_host(task, HostOpen::Plain))
    },
    tcpClose(conn_id) => {
        driver().close(conn_id);
        Ok(())
    },
    tcpCloseListener(listener_id) => {
        driver().close_listener(listener_id);
        Ok(())
    },
    parseHttpRequest(ctx, raw) => (http_parser::parse_http_request(ctx, raw)).map_err(|e| NativeError::std(StdErrorClass::Parse, e)),
    httpMethodName(ctx, method) => {
        (http_parser::HttpMethod::from_value(ctx, method)
            .map(|m| m.name().to_owned())
            .ok_or_else(|| "expected an HttpMethod".to_owned())).map_err(|e| NativeError::std(StdErrorClass::Std, e))
    },
    sendHttpResponse(ctx, conn_id, status, status_text, headers, body, cookies) => {
        let mut out = http_head::encode_head(
            ctx,
            status,
            status_text,
            headers,
            cookies,
            http_head::Framing::Length(body.len()),
        )?;
        out.extend_from_slice(body.as_bytes());

        let task = driver().write(conn_id, out);
        Ok(ctx.task_from_host(task, HostOpen::Plain))
    },
    isHttpToken(name) => http_head::is_token(name),
    isHttpFieldValue(value) => http_head::is_clean(value),
    sendHttpHead(ctx, conn_id, status, status_text, headers, cookies) => {
        let head = http_head::encode_head(
            ctx,
            status,
            status_text,
            headers,
            cookies,
            http_head::Framing::Chunked,
        )?;
        let task = driver().write(conn_id, head);
        Ok(ctx.task_from_host(task, HostOpen::Plain))
    },
    udpBind(ctx, host, port) => {
        if !ctx.check_net_listen(port) {
            return Err(NativeError::capability("net.listen", port.to_string()));
        }
        driver().udp_bind(host, port).map_err(|e| NativeError::std(StdErrorClass::Net, e.to_string()).with_str(ErrorField::Host, host).with_int(ErrorField::Port, port))
    },
    udpSendTo(ctx, socket_id, host, port, data) => {
        let bytes = payload(ctx, data).map_err(|e| NativeError::std(StdErrorClass::Net, e))?;
        let mut retries = 32;
        let sent = loop {
            match driver().udp_send_to(socket_id, &bytes, host, port) {
                Ok(n) => break n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock && retries > 0 => {
                    retries -= 1;
                    std::hint::spin_loop();
                }
                Err(e) => {
                    return Err(NativeError::std(StdErrorClass::Net, e.to_string())
                        .with_str(ErrorField::Host, host)
                        .with_int(ErrorField::Port, port));
                }
            }
        };
        Ok(sent as i64)
    },
    udpRecvFrom(ctx, socket_id, max_len) => {
        let task = driver().udp_recv(socket_id, max_len.max(0) as usize);
        Ok(ctx.task_from_host(task, HostOpen::Plain))
    },
    udpClose(socket_id) => {
        driver().udp_close(socket_id);
        Ok(())
    },
}

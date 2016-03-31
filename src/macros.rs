macro_rules! log {
    ($level:expr, $($args:tt)*) => ({
        static LOC: ::log::LogLocation = ::log::LogLocation {
            __line: line!(),
            __file: file!(),
            __module_path: module_path!()
        };
        ::systemd::journal::log($level, &LOC, &format_args!($($args)*))
    });
}

macro_rules! error {
    ($($args:tt)*) => (log!(3, $($args)*));
}

macro_rules! info {
    ($($args:tt)*) => (log!(6, $($args)*));
}

macro_rules! debug {
    ($($args:tt)*) => (log!(7, $($args)*));
}

macro_rules! log_result {
    ($e:expr) => (match $e {
        ::std::result::Result::Ok(o) => ::std::result::Result::Ok(o),
        ::std::result::Result::Err(err) => {
            error!("`{}` failed: {}", stringify!($e), err);
            ::std::result::Result::Err(err)
        }
    })
}

macro_rules! nonblock {
    ($e:expr) => (match $e {
        ::std::result::Result::Ok(o) => ::std::result::Result::Ok(Some(o)),
        ::std::result::Result::Err(ref err) if err.kind() == ::std::io::ErrorKind::WouldBlock => ::std::result::Result::Ok(None),
        ::std::result::Result::Err(err) => ::std::result::Result::Err(err),
    })
}

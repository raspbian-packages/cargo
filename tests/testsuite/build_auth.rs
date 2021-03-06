use std;
use std::collections::HashSet;
use std::io::prelude::*;
use std::net::TcpListener;
use std::thread;

use crate::support::paths;
use crate::support::{basic_manifest, project};
use bufstream::BufStream;
use git2;

// Tests that HTTP auth is offered from `credential.helper`.
#[allow(dead_code)]
fn http_auth_offered() {
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap();

    fn headers(rdr: &mut dyn BufRead) -> HashSet<String> {
        let valid = ["GET", "Authorization", "Accept"];
        rdr.lines()
            .map(|s| s.unwrap())
            .take_while(|s| s.len() > 2)
            .map(|s| s.trim().to_string())
            .filter(|s| valid.iter().any(|prefix| s.starts_with(*prefix)))
            .collect()
    }

    let t = thread::spawn(move || {
        let mut conn = BufStream::new(server.accept().unwrap().0);
        let req = headers(&mut conn);
        conn.write_all(
            b"\
            HTTP/1.1 401 Unauthorized\r\n\
            WWW-Authenticate: Basic realm=\"wheee\"\r\n
            \r\n\
        ",
        )
        .unwrap();
        assert_eq!(
            req,
            vec![
                "GET /foo/bar/info/refs?service=git-upload-pack HTTP/1.1",
                "Accept: */*",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect()
        );
        drop(conn);

        let mut conn = BufStream::new(server.accept().unwrap().0);
        let req = headers(&mut conn);
        conn.write_all(
            b"\
            HTTP/1.1 401 Unauthorized\r\n\
            WWW-Authenticate: Basic realm=\"wheee\"\r\n
            \r\n\
        ",
        )
        .unwrap();
        assert_eq!(
            req,
            vec![
                "GET /foo/bar/info/refs?service=git-upload-pack HTTP/1.1",
                "Authorization: Basic Zm9vOmJhcg==",
                "Accept: */*",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect()
        );
    });

    let script = project()
        .at("script")
        .file("Cargo.toml", &basic_manifest("script", "0.1.0"))
        .file(
            "src/main.rs",
            r#"
            fn main() {
                println!("username=foo");
                println!("password=bar");
            }
        "#,
        )
        .build();

    script.cargo("build -v").run();
    let script = script.bin("script");

    let config = paths::home().join(".gitconfig");
    let mut config = git2::Config::open(&config).unwrap();
    config
        .set_str("credential.helper", &script.display().to_string())
        .unwrap();

    let p = project()
        .file(
            "Cargo.toml",
            &format!(
                r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []

            [dependencies.bar]
            git = "http://127.0.0.1:{}/foo/bar"
        "#,
                addr.port()
            ),
        )
        .file("src/main.rs", "")
        .file(
            ".cargo/config",
            "\
        [net]
        retry = 0
        ",
        )
        .build();

    // This is a "contains" check because the last error differs by platform,
    // may span multiple lines, and isn't relevant to this test.
    p.cargo("build")
        .with_status(101)
        .with_stderr_contains(&format!(
            "\
[UPDATING] git repository `http://{addr}/foo/bar`
[ERROR] failed to load source for a dependency on `bar`

Caused by:
  Unable to update http://{addr}/foo/bar

Caused by:
  failed to clone into: [..]

Caused by:
  failed to authenticate when downloading repository
attempted to find username/password via `credential.helper`, but [..]

Caused by:
",
            addr = addr
        ))
        .run();

    t.join().ok().unwrap();
}

// Boy, sure would be nice to have a TLS implementation in rust!
#[allow(dead_code)]
fn https_something_happens() {
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap();
    let t = thread::spawn(move || {
        let mut conn = server.accept().unwrap().0;
        drop(conn.write(b"1234"));
        drop(conn.shutdown(std::net::Shutdown::Write));
        drop(conn.read(&mut [0; 16]));
    });

    let p = project()
        .file(
            "Cargo.toml",
            &format!(
                r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []

            [dependencies.bar]
            git = "https://127.0.0.1:{}/foo/bar"
        "#,
                addr.port()
            ),
        )
        .file("src/main.rs", "")
        .file(
            ".cargo/config",
            "\
        [net]
        retry = 0
        ",
        )
        .build();

    p.cargo("build -v")
        .with_status(101)
        .with_stderr_contains(&format!(
            "[UPDATING] git repository `https://{addr}/foo/bar`",
            addr = addr
        ))
        .with_stderr_contains(&format!(
            "\
Caused by:
  {errmsg}
",
            errmsg = if cfg!(windows) {
                "[..]failed to send request: [..]"
            } else if cfg!(target_os = "macos") {
                // macOS is difficult to tests as some builds may use Security.framework,
                // while others may use OpenSSL. In that case, let's just not verify the error
                // message here.
                "[..]"
            } else {
                "[..]SSL error: [..]"
            }
        ))
        .run();

    t.join().ok().unwrap();
}

// It would sure be nice to have an SSH implementation in Rust!
#[test]
fn ssh_something_happens() {
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap();
    let t = thread::spawn(move || {
        drop(server.accept().unwrap());
    });

    let p = project()
        .file(
            "Cargo.toml",
            &format!(
                r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []

            [dependencies.bar]
            git = "ssh://127.0.0.1:{}/foo/bar"
        "#,
                addr.port()
            ),
        )
        .file("src/main.rs", "")
        .build();

    p.cargo("build -v")
        .with_status(101)
        .with_stderr_contains(&format!(
            "[UPDATING] git repository `ssh://{addr}/foo/bar`",
            addr = addr
        ))
        .with_stderr_contains(
            "\
Caused by:
  [..]failed to start SSH session: Failed getting banner[..]
",
        )
        .run();
    t.join().ok().unwrap();
}

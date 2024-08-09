/// Talk to the device.
use crate::{command, read};
use tokio::{
    self,
    io::{self, AsyncWriteExt},
};

pub struct ZplPrinter {
    connection: tokio::net::TcpStream,
    status: Option<command::HostStatus>,
}

impl ZplPrinter {
    pub async fn with_address(addr: std::net::SocketAddr) -> io::Result<Self> {
        let socket = tokio::net::TcpStream::connect(addr).await?;
        Ok(Self::with_socket(socket).await)
    }

    pub async fn with_socket(socket: tokio::net::TcpStream) -> Self {
        Self {
            connection: socket,
            status: None,
        }
    }

    pub async fn request_device_status(&mut self) -> std::io::Result<&command::HostStatus> {
        let commands = command::CommandSequence(vec![
            command::ZplCommand::RequestHostIdentification,
            command::ZplCommand::RequestHostStatus,
            command::ZplCommand::RequestHostRamStatus,
        ]);

        let (mut rx, mut tx) = tokio::io::split(&mut self.connection);

        let mut lines = vec![];
        let mut buf = vec![];
        let total_expected_response_lines = commands.expected_response_lines();

        // We send-and-read in sequence. Otherwise the print-back may be unordered.. Oh my.
        for cmd in commands.0 {
            let command = command::CommandSequence(vec![cmd]);

            let expected_response_lines = command.expected_response_lines();
            let data = String::from(command).into_bytes();

            // TODO: Evaluate if these things should really run in parallel?
            tokio::try_join!(async { tx.write_all(&data).await }, async {
                for _ in 0..expected_response_lines {
                    let line = match read::line_with(&mut buf, &mut rx).await {
                        Ok(line) => line,
                        Err(err) => return Err(err),
                    };

                    lines.push(line.string);
                }

                Ok(())
            })?;
        }

        assert_eq!(lines.len() as u32, total_expected_response_lines);
        let mut info = command::HostStatus::default();

        {
            let hi = &mut info.identification;
            split_line(
                &lines[0],
                [&mut hi.model, &mut hi.version, &mut hi.dpmm, &mut hi.memory],
            );
        }

        {
            let s1 = &mut info.string1;

            split_line(
                &lines[1],
                [
                    &mut s1.a_communication,
                    &mut s1.b_paper_out,
                    &mut s1.c_pause,
                    &mut s1.d_label_length,
                    &mut s1.e_number_formats,
                    &mut s1.f_buffer_full,
                    &mut s1.g_communication_diagnostics,
                    &mut s1.h_partial_format,
                    &mut Ignore,
                    &mut s1.j_corrupt_ram,
                    &mut s1.k_temperature_low,
                    &mut s1.l_temperature_high,
                ],
            );
        }

        {
            let s2 = &mut info.string2;

            split_line(
                &lines[2],
                [
                    &mut s2.m_settings,
                    &mut Ignore,
                    &mut s2.o_head_up,
                    &mut s2.p_ribbon_out,
                    &mut s2.q_thermal_transfer_mode,
                    &mut s2.r_print_mode,
                    &mut s2.s_print_width_mode,
                    &mut s2.t_label_waiting,
                    &mut s2.u_labels_remaining,
                    &mut s2.v_format_printing,
                    &mut s2.w_number_graphics_stored,
                ],
            );
        }

        {
            let s3 = &mut info.string3;

            split_line(&lines[3], [&mut s3.x_password, &mut s3.y_static_ram]);
        }

        {
            let ram = &mut info.ram_status;
            split_line(
                &lines[4],
                [
                    &mut ram.total,
                    &mut ram.maximum_to_user,
                    &mut ram.available_to_user,
                ],
            );
        }

        Ok(self.status.insert(info))
    }

    pub async fn send(&mut self, commands: command::CommandSequence) -> std::io::Result<()> {
        // Send data to the printer
        let response_lines = commands.expected_response_lines();
        for command in String::from(commands).lines() {
            self.connection.write_all(command.as_bytes()).await?;
        }

        // Wait for incoming data
        let mut buf = vec![];
        for _ in 0..response_lines {
            let line = read::line_with(&mut buf, &mut self.connection).await?;
            eprintln!("{}", String::from_utf8_lossy(&line.string));
        }

        if response_lines == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(10_000)).await;
        }

        Ok(())
    }
}

trait FromField {
    fn fill(&mut self, st: &str);
}

struct Ignore;

fn split_line<const N: usize>(line: &[u8], data: [&mut dyn FromField; N]) {
    let Ok(line) = core::str::from_utf8(line) else {
        return;
    };

    eprintln!("{line}");
    for (st, field) in line.split(',').zip(data) {
        field.fill(st);
    }
}

impl FromField for Ignore {
    fn fill(&mut self, _: &str) {}
}

macro_rules! by_parse {
    (impl FromField for $t:ty {}) => {
        impl FromField for $t {
            fn fill(&mut self, st: &str) {
                if let Ok(val) = st.parse() {
                    *self = val;
                }
            }
        }
    };
}

by_parse!(impl FromField for u8 {});
by_parse!(impl FromField for u16 {});
by_parse!(impl FromField for u32 {});
by_parse!(impl FromField for u64 {});

impl FromField for bool {
    fn fill(&mut self, st: &str) {
        *self = match st.parse::<u8>() {
            Ok(0) => false,
            Ok(1) => true,
            _ => return,
        };
    }
}

impl FromField for String {
    fn fill(&mut self, st: &str) {
        self.replace_range(.., st);
    }
}

use core::fmt::Write;
use menu::{Item, ItemType, Menu, Parameter};

pub const CLI_QUEUE_SIZE: usize = 512;

pub struct CliOutput<const N: usize> {
    write_queue: heapless::spsc::Producer<'static, u8, N>,
}

impl<const N: usize> CliOutput<{ N }> {
    pub fn new(write_queue: heapless::spsc::Producer<'static, u8, N>) -> CliOutput<N> {
        CliOutput { write_queue }
    }
}

impl<const N: usize> core::fmt::Write for CliOutput<{ N }> {
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        for byte in s.bytes() {
            if self.write_queue.enqueue(byte).is_err() {
                return Err(core::fmt::Error);
            }
        }
        Ok(())
    }
}

pub const ROOT_MENU: Menu<CliOutput<CLI_QUEUE_SIZE>> = Menu {
    label: "root",
    items: &[&Item {
        item_type: ItemType::Callback {
            function: select_foo,
            parameters: &[
                Parameter::Mandatory {
                    parameter_name: "a",
                    help: Some("This is the help text for 'a'"),
                },
                Parameter::Optional {
                    parameter_name: "b",
                    help: None,
                },
                Parameter::Named {
                    parameter_name: "verbose",
                    help: None,
                },
                Parameter::NamedValue {
                    parameter_name: "level",
                    argument_name: "INT",
                    help: Some("Set the level of the dangle"),
                },
            ],
        },
        command: "foo",
        help: Some(
            "Makes a foo appear.
This is some extensive help text.
It contains multiple paragraphs and should be preceeded by the parameter list.
",
        ),
    }],
    entry: Some(enter_root),
    exit: Some(exit_root),
};

fn select_foo<const N: usize>(
    _menu: &Menu<CliOutput<N>>,
    _item: &Item<CliOutput<N>>,
    args: &[&str],
    context: &mut CliOutput<N>,
) {
    writeln!(context, "In select_bar. Args = {:?}", args).unwrap();
}

fn enter_root<const N: usize>(_menu: &Menu<CliOutput<N>>, context: &mut CliOutput<N>) {
    writeln!(context, "In enter_root").unwrap();
}

fn exit_root<const N: usize>(_menu: &Menu<CliOutput<N>>, context: &mut CliOutput<N>) {
    writeln!(context, "In exit_root").unwrap();
}

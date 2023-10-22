mod get;
pub use get::Get;

mod unknown;
pub use unknown::Unknown;

use crate::{db::Db, parse::Parse, shutdown::Shutdown, Connection, Frame};

/// Enumeration of supported Redis commands
///
/// Methods called on `Command` are delegated to the command implementaiton.
#[derive(Debug)]
pub enum Command {
    Get(Get),
    Unknown(Unknown),
}

impl Command {
    /// Parse a command from a received frame.
    ///
    /// The `Frame` must represent a Redis command supported by `mini-redis` and
    /// be the array variant.
    ///
    /// # Returns
    ///
    /// On success, the command value is returned, otherwise `Err` is returned.
    pub fn from_frame(frame: Frame) -> crate::Result<Command> {
        // The frame value is decorated with `Parse`. `Parse` provides a
        // "cursor" like API which makes parsing the command easier.
        //
        // The frame value must be an array variant. Any other frame variants
        // result in an error being returned.
        let mut parse = Parse::new(frame)?;

        // All redis commands begin with command name as a string. The name
        // is read and converted to lower cases in order to do case sensitive
        // matching.
        let command_name = parse.next_string()?.to_lowercase();

        // Match the command name, delegating the rest of the parsing to the
        // specific command.
        let command = match &command_name[..] {
            "get" => Command::Get(Get::parse_frames(&mut parse)?),
            _ => {
                // The command is not recognized and an Unknown command is 
                // returned. 
                //
                // `return` is called here to skip the `finish()` call below. As 
                // the command is not recognized, there is most likey 
                // unconsumed fields remaining in the `Parse` instance. 
                return Ok(Command::Unknown(Unknown::new(command_name)));
            }
        };

        // Check if there is any remaining unconsumed fields in the `Parse` 
        // value. If fields remain, this indicates an unexpected frame format 
        // and an error is returned. 
        parse.finish()?;

        // The comamnd has been successfully parsed
        Ok(command)
    }

    /// Apply the command to the specified `Db` instance.
    ///
    /// The response is written to `dst`. This is called by the server in order
    /// to execute a received command.
    pub(crate) async fn apply(
        self,
        db: &Db,
        dst: &mut Connection,
        _shutdown: Shutdown,
    ) -> crate::Result<()> {
        use Command::*;

        match self {
            Get(cmd) => cmd.apply(db, dst).await,
            Unknown(cmd) => cmd.apply(dst).await,
        }
    }

    /// Returns the command name
    pub(crate) fn get_name(&self) -> &str {
        match self {
            Command::Get(_) => "get",
            Command::Unknown(cmd) => cmd.get_name(),
        }
    }
}

use opml::OPML;

use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponseFollowup};
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{ResolvedOption, ResolvedValue};
use serenity::model::prelude::CommandInteraction;

use crate::database::Database;
use crate::structs::feed::ServerData;

pub async fn run(
    options: &[ResolvedOption<'_>],
    interaction: &CommandInteraction,
) -> CreateInteractionResponseFollowup {
    let followup = CreateInteractionResponseFollowup::new();

    let mut db = Database::load(None);
    let guild_id = interaction.guild_id.unwrap().to_string();
    let ResolvedValue::Attachment(file) = &options.get(0).unwrap().value else { return followup.content("String value not found"); };

    let opml_file = match file.download().await {
        Ok(content) => content,
        Err(_) => return followup.content("Cannot download attachment."),
    };
    let ompl_document = match OPML::from_str(&String::from_utf8_lossy(&opml_file)) {
        Ok(doc) => doc,
        Err(err) => {
            let reason = match err {
                opml::Error::BodyHasNoOutlines => "OPML file has no RSS feed.",
                opml::Error::IoError(_) => "An error occurred while reading OPML file. If this keeps happening, please contact to a developer.",
                opml::Error::UnsupportedVersion(_) => "Unsupported version or out-of-standard OPML file.",
                opml::Error::XmlError(_) => "An error occurred while parsing OPML file. If this keeps happening, please contact to a developer."
            };

            return followup.content(format!("Cannot import OPML file. {reason}"));
        }
    };

    let mut feeds_list = vec![];
    for outline in ompl_document.body.outlines {
        if outline.outlines.is_empty() {
            feeds_list.push(outline.xml_url.unwrap())
        } else {
            for outline in outline.outlines {
                feeds_list.push(outline.xml_url.unwrap())
            }
        }
    }

    let data: ServerData;
    if let Some(current_data) = db.get::<ServerData>(&guild_id) {
        let current_feeds_list = current_data.feeds_list.unwrap_or_default();
        data = ServerData {
            feeds_list: Some([current_feeds_list.as_slice(), feeds_list.as_slice()].concat()),
            ..current_data
        };
    } else {
        data = ServerData {
            feeds_list: Some(feeds_list),
            ..Default::default()
        };
    }

    db.set(&guild_id, &data).unwrap();
    followup.content("Imported.")
}

pub fn register() -> CreateCommand {
    CreateCommand::new("import")
        .description("Import RSS list from an OPML file.")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Attachment, "file", "OPML file.")
                .required(true),
        )
}

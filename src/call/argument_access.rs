//! Stripping an argument requires knowing analytics owns it; reading one only
//! requires not knowing that the tool does.

use crate::options::Options;
use crate::tools::cache::ToolFacts;
use crate::tools::ownership::ArgumentFlags;

pub(super) struct ArgumentAccess {
    pub strip: ArgumentFlags,
    pub read: ArgumentFlags,
}

pub(super) fn resolve(options: &Options, tool: Option<&ToolFacts>) -> ArgumentAccess {
    let enabled = ArgumentFlags {
        context: options.context,
        conversation_id: options.enable_conversation_id,
        llm_model: options.capture_model,
    };
    let owned = tool.map(|tool| tool.ownership.arguments);
    ArgumentAccess {
        strip: enabled.and(owned.unwrap_or(ArgumentFlags::all(false))),
        read: enabled.and(owned.unwrap_or(ArgumentFlags::all(true))),
    }
}

--!strict
-- BN5's attachments (actor object #5, BN6's code: @exelib/attachment/attachment), BN5's kind. What it shows is a look
-- (`attachment.look { ... }`); `attachment.attach(owner, look, slot, { anim = ... })`.

local common = require("@exelib/attachment/attachment")

local attachment = {}

-- An attachment's look.
attachment.look = common.look

attachment.kind = common.make_kind("bn5:attachment")

-- Attach `look` to `owner`, stored in `which`.
function attachment.attach(owner: Object, look: AttachmentLook, which: Slot, options: AttachmentOptions?): Object?
    return common.attach(attachment.kind, owner, look, which, options)
end

return attachment

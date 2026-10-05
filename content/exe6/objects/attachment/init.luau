--!strict
-- Attachments (the original's actor object #5, `sub_80B8CD8`): the shared
-- attachment (@exelib/attachment/attachment), EXE6's kind. What it shows is
-- a look (`attachment.look { ... }`), a record its user defines; how it
-- animates is the user's too (`attachment.attach(owner, look, slot, {
-- anim = ... })`).

local common = require("@exelib/attachment")

local attachment = {}

-- An attachment's look (see AttachmentLook in types.d.luau).
attachment.look = common.look

attachment.kind = common.make_kind("attachment")

-- `sub_80B8E30`: attach `look` to `owner`, stored in `which`.
function attachment.attach(owner: Object, look: AttachmentLook, which: Slot, options: AttachmentOptions?): Object?
    return common.attach(attachment.kind, owner, look, which, options)
end

return attachment

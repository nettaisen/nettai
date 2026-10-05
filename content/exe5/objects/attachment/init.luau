--!strict
-- EXE5's attachments (actor object #5, EXE6's code: @exelib/attachment/attachment), EXE5's kind. What it shows is a look
-- (`attachment.look { ... }`); `attachment.attach(owner, look, slot, { anim = ... })`.

local common = require("@exelib/attachment")

local attachment = {}

-- An attachment's look.
attachment.look = common.look

-- EXE5's lift (0x080B9AB8): MegaMan's (NameID 0x180: his identities') in Hub Style (NaviStats +0x4C of the
-- attachment's side), 1; else the look's (every EXE5 row's is 0). EXE5's follow (0x080B9B50) adds its byte, unsigned,
-- to Y and Z, where EXE6's subtracts its signed byte: in the shared code's terms, -1.
attachment.kind = common.make_kind("attachment", function(owner: Object, side: number): number?
    local identity = owner.identity
    if identity ~= nil and identity.class == "megaman" and battle.navi(side).hub_style ~= 0 then
        return -1
    end
    return nil
end)

-- Attach `look` to `owner`, stored in `which`.
function attachment.attach(owner: Object, look: AttachmentLook, which: Slot, options: AttachmentOptions?): Object?
    return common.attach(attachment.kind, owner, look, which, options)
end

return attachment

--!strict
-- BN5's attachments (actor object #5, BN6's code: @common/attachment/attachment), BN5's kind. What it shows is a look
-- (`attachment.look { ... }`); `attachment.attach(owner, look, slot, { anim = ... })`.

local common = require("@common/attachment/attachment")

local attachment = {}

-- An attachment's look.
attachment.look = common.look

-- BN5's lift (0x080B9AB8): MegaMan's (NameID 0x180: his identities') in Hub Style (NaviStats +0x4C of the
-- attachment's side), 1 pixel; else the look's.
attachment.kind = common.make_kind("bn5:attachment", function(owner: Object, side: number): number?
    local identity = owner.identity
    if identity ~= nil and identity.class == "megaman" and battle.navi(side).hub_style ~= 0 then
        return 1
    end
    return nil
end)

-- Attach `look` to `owner`, stored in `which`.
function attachment.attach(owner: Object, look: AttachmentLook, which: Slot, options: AttachmentOptions?): Object?
    return common.attach(attachment.kind, owner, look, which, options)
end

return attachment

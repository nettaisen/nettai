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

-- EXE6's lift (`sub_80B8CF8`): a Cross's identity (NameIDs 0x1AC to 0x1B5: a form of the side's navi's form lists,
-- its `forms.gregar` and `forms.falzar`) while the side's navi isn't in its base form (NaviStats +0x2C), 8 pixels;
-- else the look's. (DblBeast's beasts wear Beast Out's identity: never lifted, whatever the side's form.)
attachment.kind = common.make_kind("attachment", function(owner: Object, side: number): number?
    local identity = owner.identity
    local stats = battle.navi(side)
    if identity == nil or stats.form.base then
        return nil
    end
    local sets: { [string]: FormSet } = (stats.navi.forms or {}) :: any
    for _, set in sets do
        for _, form in set.form_list do
            if form.identity == identity then
                return -8
            end
        end
    end
    return nil
end)

-- `sub_80B8E30`: attach `look` to `owner`, stored in `which`.
function attachment.attach(owner: Object, look: AttachmentLook, which: Slot, options: AttachmentOptions?): Object?
    return common.attach(attachment.kind, owner, look, which, options)
end

-- A shooting chip's `hold` with this kind: by the holder's AI index from `arm_anims` (@exelib/attachment's
-- `holder`).
function attachment.holder(arm_anims: { number }, players_only: boolean?): (me: Object, look: AttachmentLook, which: Slot) -> ()
    return common.holder(attachment.kind, arm_anims, players_only)
end

return attachment

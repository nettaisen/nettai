--!strict
-- The sword a navi holds while it slashes (attachment kinds 3, 0x19..0x1C):
-- which one, its animation and palette (`sub_80EBB34`, `sub_80EBAE8`,
-- `sub_80EBB78`), as the sword actions spawn it into the navi's first
-- related slot.

local attachment = require("../objects/attachment/attachment")

local sword = {}

-- `byte_80EBB64`: the sword by the attack's variant (a sword chip's
-- subtype): the plain sword, and the element swords for 0xC..0xF.
local KINDS = {
    [0x00] = 0x03,
    [0x01] = 0x03,
    [0x02] = 0x03,
    [0x03] = 0x03,
    [0x04] = 0x03,
    [0x05] = 0x03,
    [0x06] = 0x03,
    [0x07] = 0x03,
    [0x08] = 0x03,
    [0x09] = 0x03,
    [0x0A] = 0x03,
    [0x0B] = 0x03,
    [0x0C] = 0x19,
    [0x0D] = 0x1A,
    [0x0E] = 0x1B,
    [0x0F] = 0x1C,
    [0x10] = 0x00,
    [0x11] = 0x03,
    [0x12] = 0x03,
    [0x13] = 0x00,
}

-- `sub_80EBAE8`: the sword's animation, by who holds it: none for a virus;
-- an enemy navi's AI index below 13; a player's AI index (19 shows 14) or,
-- for MegaMan himself, 12 in the Beast forms (13 in Falzar's and in the
-- later Beast Crosses and Beast Over).
local function animation(navi: Object): number
    local kind, ai = navi.actor_type, navi.ai_index
    if kind == "virus" then
        return 0
    elseif kind == "navi" then
        return if ai < 13 then ai else 0
    end
    if ai ~= 0 then
        return if ai == 19 then 14 else ai
    end
    local form = battle.navi(navi.alliance).form
    if form < 0x0B then
        return 0
    elseif form == 0x0C or (form ~= 0x17 and form > 0x11) then
        return 13
    end
    return 12
end

-- `sub_80EBB78`: the palette addition: 12 for an enemy navi of AI index
-- 0x12 in its fourth version (AIData+0x16, its NameID record's).
local function palette(navi: Object): number
    if navi.actor_type ~= "navi" or navi.ai_index ~= 0x12 then
        return 0
    end
    local record = battle.navi_record(navi.name_id)
    if record == nil then
        error(string.format("NameID %#x's record is not in the content (sub_80EBB78)", navi.name_id))
    end
    return if record.version == 4 then 12 else 0
end

-- `sub_80EBB34` .. `sub_80B8E30`: the sword for the attack's variant, in
-- the navi's first related slot.
function sword.raise(navi: Object, variant: number): Object?
    local kind = KINDS[variant]
    if kind == nil then
        error(string.format("sword variant %#x reads past byte_80EBB64 (sub_80EBB34)", variant))
    end
    return attachment.spawn(navi, { kind = kind, anim = animation(navi), palette_add = palette(navi) }, "related")
end

return sword

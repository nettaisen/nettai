--!strict
-- The blade SlashCross's charged slash (action 0x41, its weapons 0x11 and
-- 0x12) raises: which one by the attack's variant (`byte_80EBB64`), its
-- animation and palette (`sub_80EBAE8`, `sub_80EBB78`), in the navi's first
-- related slot. (The sword chips name their blades themselves, lib/swords;
-- this goes into lib/swords when SlashCross's weapons convert.)

local attachment = require("../objects/attachment/attachment")
local parts = require("./swords/parts")

local sword = {}

-- `byte_80EBB64`: the blade (the pack data's attachment row) by the
-- attack's variant: the plain sword, the element swords for 0xC to 0xF,
-- and row 0 for 0x10 and 0x13.
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

-- `sub_80EBB34` .. `sub_80B8E30`: the blade for the attack's variant, in
-- the navi's first related slot.
function sword.raise(navi: Object, variant: number): Object?
    local kind = KINDS[variant]
    if kind == nil then
        error(string.format("sword variant %#x reads past byte_80EBB64 (sub_80EBB34)", variant))
    end
    return attachment.spawn(
        navi,
        { kind = kind, anim = parts.blade_anim(navi), palette_add = parts.blade_palette(navi) },
        "related"
    )
end

return sword

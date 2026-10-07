--!strict
-- ProtoMan, the navi of EXE4's ProtoMan chips (EXE4's actor object #0x3C, 0x080C6ED4), and each chip's controller,
-- which brings him: EXE6's and EXE5's (@exelib/protoman/navi; the same code) with EXE4's look (his sprite, 08-07, in
-- his chip's palette, 0x0800B950: two palettes a variant; his sword, attachment row 3, 0c-00, in its animation 7,
-- animating while dimmed; his slash, effect row 0x5E, 0c-14) and its differences: his spawner (0x080C721A) keeps
-- neither his user, nor its facing, nor the element, his height his side; he lands in front of an enemy navi on a solid
-- panel no body or object of his side's is on (0x080C70CC, by side), his user's panel no exception; and his slash
-- strikes as it starts (0x080C71AA), his sword away as it ends.

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/protoman/navi")
local navi_chips = require("../../lib/navi_chips")

-- What a panel he lands on must be free of, by his side (0x080C70CC).
local OCCUPIED = { [0] = 0x0788_0080, 0x0B88_0080 }

-- 0x080CD6E0, as his slash asks (attachment row 3, his sword's animation, animating while dimmed).
local function attach(owner: Object, look: AttachmentLook, which: Slot, options: AttachmentOptions?): Object?
    local anim = if options then options.anim or 0 else 0
    local while_dimmed = options ~= nil and options.while_dimmed == true
    return attachment.attach(owner, look, which, anim, while_dimmed)
end

local protoman = common.make {
    id = "protoman/navi",
    appear_sound = asset.sound("appear"),
    swing_sound = asset.sound("sword-swing"),
    attachment = attachment.kind,
    target = collision.hits_navis,
    self_type = collision.slash,
    summoned_alone = true,
    occupied = OCCUPIED,
    user_panel_free = false,
    slash_at_once = true,
    attach = attach,
}

-- His sword (attachment row 3) and his slash (effect row 0x5E).
local SWORD = attachment.look { sprite = asset.sprite("sword"), palette = 0 }
local SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 }

-- What each ProtoMan chip states for its own controller: the controller's id (the original's effect object #0x42,
-- 0x080E6A18, for all six navis' chips: compat maps each to it) and his palette (0x0800B950 by the chip's level: two
-- palettes a variant).
export type ChipSpec = { id: string, palette: number }

-- A ProtoMan chip's controller: the user warps out, ProtoMan (in the chip's look), the user warps back in.
local function controller(chip: ChipSpec): NaviChipController
    local summon = protoman.summon_with(common.look {
        sprite = asset.sprite("protoman"),
        palette = chip.palette,
        sword = SWORD,
        sword_anim = 7,
        slash = SLASH,
        spawner_address = 0,
    })
    -- 0x080C721A: ProtoMan for the controller's user, with the damage plus the bonus.
    local function spawn(c: Object, user: Object, damage: number): Object?
        return summon(user, c, { panel_x = c.panel_x, panel_y = c.panel_y, element = c.element, damage = damage })
    end
    return navi_chips.controller { id = chip.id, name = "ProtoMan", spawn = spawn, steps = navi_chips.SHARED }
end

return { controller = controller }

--!strict
-- ProtoMan, the navi of EXE4's ProtoMan chips (EXE4's actor object #0x3C, 0x080C6ED4), whom the shared navi chip
-- controller (objects/navi_chip_controller, its entry 8) brings: EXE6's and EXE5's (@exelib/protoman/navi; the same
-- code) with EXE4's look (his sprite, 08-07, in his chip's palette, 0x0800B950: two palettes a variant; his sword,
-- attachment row 3, 0c-00, in its animation 7, animating while dimmed; his slash, effect row 0x5E, 0c-14) and its
-- differences: his spawner (0x080C721A) keeps neither his user, nor its facing, nor the element, his height his side;
-- he lands in front of an enemy navi on a solid panel no body or object of his side's is on (0x080C70CC, by side), his
-- user's panel no exception; and his slash strikes as it starts (0x080C71AA), his sword away as it ends.

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/protoman/navi")
local navi_chips = require("../../lib/navi_chips")

-- What a panel he lands on must be free of, by his side (0x080C70CC).
local OCCUPIED = { [0] = 0x0788_0080, 0x0B88_0080 }
-- His palettes a variant (0x0800B9B1's entry 7).
local PALETTES = 2

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

-- His look for a chip of `level` (0, an SP's 3, a DS's 4): his palette by it.
local summons: { [number]: (user: Object, controller: Object, spec: NaviChipSpec) -> Object? } = {}
for _, level in { 0, 3, 4 } do
    summons[level] = protoman.summon_with(common.look {
        sprite = asset.sprite("protoman"),
        palette = navi_chips.palette(PALETTES, level),
        sword = attachment.look { sprite = asset.sprite("sword"), palette = 0 },
        sword_anim = 7,
        slash = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 },
        spawner_address = 0,
    })
end

-- 0x080C721A: ProtoMan for the controller's user, with the damage plus the bonus.
local function spawn(controller: Object, user: Object, damage: number): Object?
    local level = (controller.state :: any).level :: number
    local summon = summons[level]
    if not summon then
        error(string.format("ProtoMan's controller has a level %d no chip gives", level))
    end
    return summon(
        user,
        controller,
        { panel_x = controller.panel_x, panel_y = controller.panel_y, element = controller.element, damage = damage }
    )
end

return { kind = protoman.kind, spawn = spawn }

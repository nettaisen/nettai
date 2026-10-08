--!strict
-- BurnMan, the navi of EXE4's BurnMan chips (EXE4's actor object #0x23, 0x080BE168: EXE4's own), and each chip's
-- controller, which brings him (effect object #0x18, 0x080E3AE8: lib/navi_chips' short course). He appears on his
-- user's panel and, 21 ticks later (gone if it isn't solid), raises his pillars (./pillar) two columns ahead, a row above
-- and below; 20 ticks later he winds up (animation 4) and, 50 ticks on, breathes his flame (./flame) over the three
-- panels ahead of him (animation 5), roaring each 16 ticks for 60 ticks; then he goes in a puff. His pillars' and
-- flame's hits linger on their panels (../../objects/lingering_hit: fire, the fire spark, hitting navis).

local collision = require("../../rules/collision")
local flame_kind = require("./flame")
local lingering_hit = require("../../objects/lingering_hit")
local navi_chips = require("../../lib/navi_chips")
local pillar_kind = require("./pillar")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local burnman = {}

type State = {
    -- The controller waiting for him (the flag it gave him, his +0x4C).
    controller: Object?,
    -- His chip's palette.
    palette: number,
}

local SPRITE = asset.sprite("burnman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_PILLARS = asset.sound("tops")
local SOUND_BREATH = asset.sound("rage-roar")
-- The puff he goes in (effect row 0x13).
local PUFF = new.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 }
local SOLID = 0x10
local FIRE = 1

local HITS = lingering_hit.make("burnman/hit")

-- A lingering hit `dx`, `dy` off `owner`'s panel (0x080D4002): its region the panel, the fire spark, an attack hitting
-- navis (modifier 3), fire, 16 pixels up, for `ticks`; kept in `owner`'s `hits` slot `slot`, if any.
local function hit(owner: Object, dx: number, dy: number, ticks: number, slot: number?)
    HITS.spawn(owner, dx, dy, {
        region = regions.single,
        hit_effect = sparks.fire,
        target = collision.hits_navis,
        self_type = collision.attack,
        hit_mod = 3,
        damage = bit32.bor(owner.damage, bit32.lshift(owner.stamina, 16)),
        z = 0x10_0000,
        ticks = ticks,
        element = FIRE,
        holder = if slot then owner else nil,
        slot = slot,
    })
end

local PILLAR = pillar_kind.make(function(owner: Object, dx: number, dy: number)
    hit(owner, dx, dy, 1)
end)
local FLAME = flame_kind.make(function(owner: Object, dx: number, dy: number, slot: number)
    hit(owner, dx, dy, 60, slot)
end)

local APPEAR, ATTACK = 0x0, 0x4

-- 0x080BE228: 21 ticks; then, on a solid panel, his pillars two columns ahead, a row above and below (the lower one
-- its parameter 2 1), with their sound; 20 ticks to his windup. Else he goes.
local function appear(me: Object)
    local s = me.state :: State
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    if not field.check(me.panel_x, me.panel_y, SOLID, 0) then
        me:set_lifecycle("destroy")
        return
    end
    local x = me.panel_x + 2 * me.facing
    PILLAR.spawn(me, x, me.panel_y - 1, s.palette, false)
    PILLAR.spawn(me, x, me.panel_y + 1, s.palette, true)
    battle.play_sound(SOUND_PILLARS)
    me.timer = 20
    me:set_action(ATTACK)
end

-- Action 4, phase 0 (0x080BE2AC): 20 ticks; then his windup (animation 4), 50 ticks.
local function wait(me: Object)
    me.timer = int.u16(me.timer - 1)
    if me.timer ~= 0 then
        return
    end
    me.anim = 4
    me.timer = 50
    me.phase, me.phase_init = 4, 0
end

-- Action 4, phase 4 (0x080BE2C4): then his flame, 8 pixels short of the panel ahead's center (animation 5); 60
-- ticks of roaring.
local function wind_up(me: Object)
    me.timer = int.u16(me.timer - 1)
    if me.timer ~= 0 then
        return
    end
    local cx, cy = field.center(me.panel_x + me.facing, me.panel_y)
    FLAME.spawn(me, Vec3.new(int.i32(cx - me.facing * 0x8_0000), cy, 0))
    me.anim = 5
    me.timer2 = 1
    me.timer = 60
    me.phase, me.phase_init = 8, 0
end

-- Action 4, phase 8 (0x080BE304): a roar each 16 ticks; 60 ticks, then he goes.
local function breathe(me: Object)
    me.timer2 = int.u16(me.timer2 - 1)
    if me.timer2 == 0 then
        battle.play_sound(SOUND_BREATH)
        me.timer2 = 16
    end
    me.timer = int.u16(me.timer - 1)
    if me.timer == 0 then
        me:set_lifecycle("destroy")
    end
end

local PHASES: { [number]: (Object) -> () } = { [0] = wait, [4] = wind_up, [8] = breathe }

local function attack(me: Object)
    local f = PHASES[me.phase]
    if not f then
        error(string.format("BurnMan's attack's phase %#x reads past its table (0x080BE2A0)", me.phase))
    end
    f(me)
end

-- 0x080BE20E: his action.
local function tick(me: Object)
    if me.action == APPEAR then
        appear(me)
    elseif me.action == ATTACK then
        attack(me)
    else
        error(string.format("BurnMan's action %#x reads past his table (0x080BE220)", me.action))
    end
end

-- 0x080BE1AC: his sprite (08-15) with his shadow, shown, in animation 0, his chip's palette, facing his side's way, on
-- his panel; his sound; 21 ticks to his pillars; then his first tick.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = s.palette
    sprite:set_flip(me.alliance)
    me:set_coordinates_from_panel()
    battle.play_sound(SOUND_APPEAR)
    me.timer = 20
    me:set_lifecycle("update")
    tick(me)
end

burnman.kind = new.kind {
    id = "burnman/navi",
    pool = "actor",
    state = { controller = "object", palette = "u8" },
    -- 0x080BE168: by his state, then his sprite, while dimmed too.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x080BE18C: a puff 16 pixels above him (effect row 0x13); the controller moves on; he is freed.
            local p = me.pos
            battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), PUFF)
            local controller = (me.state :: State).controller
            if controller then
                navi_chip.navi_left(controller)
            end
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BE328: BurnMan for the controller's user, with the damage plus the bonus, in the chip's palette. (His position
-- is the spawner's registers: the row, the element and 0.)
local function spawn(controller: Object, damage: number, palette: number): Object?
    local o = battle.spawn(burnman.kind, Vec3.new(controller.panel_y, controller.element, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = controller.panel_x, controller.panel_y
    o.element = controller.element
    o.alliance = controller.alliance
    o:set_damage_word(damage)
    local s = o.state :: State
    s.controller = controller
    s.palette = palette
    return o
end

-- What each BurnMan chip states for its own controller: the controller's id (the original's effect object #0x18 for
-- all three: compat maps each to it) and his palette (0x080BE1D8 by the chip's level: 0, the SP's 2, the DS's 4).
export type ChipSpec = { id: string, palette: number }

-- A BurnMan chip's controller: the short course (the user warps out, 11 ticks, BurnMan, 21 ticks after he left, the
-- user warps back in).
local function controller(chip: ChipSpec): NaviChipController
    return navi_chips.controller {
        id = chip.id,
        name = "BurnMan",
        spawn = function(c: Object, _user: Object, damage: number): Object?
            return spawn(c, damage, chip.palette)
        end,
        short = navi_chips.SHORT,
    }
end

return { controller = controller }

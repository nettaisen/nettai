--!strict
-- Count's lance (the original's attack object #0x0D; the Japanese
-- games' code, at 0x080C9498 in EXE6 Falzar, 0x080CACF8 in EXE6 Gregar:
-- the US games point the slot at a placeholder). It waits 4096 pixels up
-- over its panel, then drops on it and strikes: a thrown attack on its
-- panel, for 30 ticks or until it hits. Count's (chips/count/navi)
-- strike at once and while dimmed; the other kind of lance, which first
-- highlights its panel and stands still while dimmed, is the Japanese
-- games' Count navi AI's (attack object #0x0C, out of a netbattle's
-- reach): ported, unverified. See docs/engine/chips.md §3.6.37.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local lance = {}

type Spec = CountLance

export type State = {
    while_dimmed: boolean,
    warn_ticks: number,
}

local SPRITE = asset.sprite("bn6:count")
local SOUND_STRIKE = asset.sound("bn6:elmnt-vine")

-- Its animation, and its height until it strikes (whole pixels).
local ANIM = 0xC
local WAITING_HEIGHT = 0x1000

-- How long the strike lasts; its hit modifier.
local STRIKE_TICKS = 0x1E
local HIT_MOD = 3

-- Its actions.
local WARN, STRIKE = 0x0, 0x4

-- Z's whole part (a halfword store), its fraction kept.
local function set_height(me: Object, pixels: number)
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.bor(bit32.band(p.z, 0xFFFF), bit32.lshift(bit32.band(pixels, 0xFFFF), 16)))
end

-- The init: on its panel, 4096 pixels up; collision on, region off; then
-- its first action: the strike for Count's, the warning for the
-- other's. Without a collision slot it is freed at once.
local function init(me: Object)
    local s = me.state :: State
    me:set_coordinates_from_panel()
    set_height(me, WAITING_HEIGHT)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = ANIM
    me.anim_loaded = ANIM
    sprite:set_animation(ANIM)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown, collision.hits_navis, HIT_MOD)
    local c = me.collision
    c:set_hit_effect(sparks.plain)
    c:set_region(nil)
    c:present()
    -- A byte store of the state, the phase cleared, then the action.
    me.lifecycle = "update"
    me.phase, me.phase_init = 0, 0
    me.action = if s.while_dimmed then STRIKE else WARN
end

-- The warning: its panel highlighted 4 ticks out of 8 for `warn_ticks`
-- ticks (at least one), then the strike.
local function warn(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = (me.state :: State).warn_ticks
        me.timer2 = 0
        return
    end
    local counted = bit32.band(me.timer2 + 1, 0xFFFF)
    me.timer2 = counted
    if bit32.band(counted, 4) == 0 then
        field.highlight(me.panel_x, me.panel_y)
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.action = STRIKE
    me.phase, me.phase_init = 0, 0
end

-- The strike: down on its panel (Z's whole part 0), its animation from
-- the start, its region on, the sound; 30 ticks, its region off once it
-- has hit; then it goes.
local function strike(me: Object)
    local c = me.collision
    if me.phase == 0 then
        me.phase = 4
        set_height(me, 0)
        me.anim = ANIM
        me.anim_loaded = 0xFF
        c:set_region(regions.single)
        battle.play_sound(SOUND_STRIKE)
        me.timer = STRIKE_TICKS
        return
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    c:set_region(nil)
    me:set_lifecycle("destroy")
end

-- Each tick: resolve the hits; when the battle is over it goes; once it
-- has hit, its region goes; the other kind stands still while dimmed;
-- then its action, and its collision presented.
local function tick(me: Object)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    if not s.while_dimmed and battle.dimmed() then
        return
    end
    local a = me.action
    if a == WARN then
        warn(me)
    elseif a == STRIKE then
        strike(me)
    else
        error(string.format("Count's lance: action %#x reads past its table", a))
    end
    c:present()
end

lance.kind = define.kind {
    id = "bn6:count/lance",
    pool = "attack",
    state = { while_dimmed = "bool", warn_ticks = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        if (me.state :: State).while_dimmed then
            me:update_sprite_while_dimmed()
        else
            me:update_sprite_while_paused()
        end
    end,
}

-- The spawner (0x080C9614 in EXE6 Falzar): a lance of `owner`'s side on
-- (x, y) with the damage word `damage`, striking as `spec` says; it runs
-- while dimmed (its own check stops the other kind). Its position is what
-- the spawner's registers held: the row, the element and `z`, of which its
-- init keeps the fraction.
function lance.spawn(owner: Object, x: number, y: number, element: number, damage: number, z: number, spec: Spec): Object?
    local o = battle.spawn(lance.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
        local s = o.state :: State
        s.while_dimmed, s.warn_ticks = spec.while_dimmed, spec.warn_ticks
    end
    return o
end

return lance

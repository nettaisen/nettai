--!strict
-- TopMan's tops (attack object #0x29, 0x080D1DD0: EXE4's own), which he sets on the panels around his (./navi). Each
-- appears on its panel (gone at once unless it is valid and solid), TopMan's sprite in its animation 8, a body of row
-- 10 hitting navis (the heavy spark); waits 30 ticks, 15 more for each before it, then spins off diagonally (the
-- tops ahead and behind, up and down: its number's), its panel following it; it goes, its region off, over a panel
-- that isn't solid (with a puff) or past the field's right edge, as the battle ends, or once it hit a body; a hit on
-- it ends its region until it moves to another panel, and ends it while it waits.

local collision = require("../../rules/collision")
local navi_chips = require("@exelib/navi_chips/navi")
local sparks = require("../../lib/sparks")
local regions = require("@exelib/regions")

local top = {}

export type State = {
    -- The ticks between its row crossings (Param2: 18), half of them (Param3), and the ticks left (Param4).
    period: number,
    half: number,
    count: number,
    -- Its panel when a hit ended its region (+0x60: x | y << 8).
    hit_at: number,
}

local SPRITE = asset.sprite("topman")
local SOUND_APPEAR = asset.sound("tops")
local SOUND_SPIN = asset.sound("boomerang")
-- The puff it goes in over a panel that isn't solid (effect row 0x15).
local PUFF = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 }
local SOLID = 0x10
local WAIT, SPIN = 0x0, 0x4
-- Its period, and its speeds across and along (0x080D1FA0: 18 ticks a column of 40 pixels, a row of 24, by
-- `Div`).
local PERIOD = 18
local VX = (0x28_0000 + bit32.lshift(PERIOD, 15) + bit32.lshift(PERIOD, 14)) // PERIOD
local VY = (0x18_0000 + bit32.lshift(PERIOD, 15)) // PERIOD
-- What a hit on it does: a body's (bits 0x03800000) ends it; another's (0x3C000000) ends its region.
local ENDS, STOPS = 0x0380_0000, 0x3C00_0000
-- Past this X it is off the field's right edge (0x080B6C7C).
local RIGHT_EDGE = 0x8C_0000

-- 0x080B6C90: it moves across by its velocity, and along: as it passes its panel's row center it settles on its
-- panel's center and counts its period again; halfway through the count its panel follows it.
local function move(me: Object)
    local s = me.state :: State
    local p, v = me.pos, me.vel
    local x = int.i32(p.x + v.x)
    me.pos = Vec3.new(x, p.y, p.z)
    local _, cy = field.center(me.panel_x, me.panel_y)
    if v.y == 0 then
        me:set_panel_from_coordinates()
        return
    end
    local old = p.y
    local y = int.i32(old + v.y)
    me.pos = Vec3.new(x, y, p.z)
    if cy ~= old and navi_chips.between(y, old, cy) then
        me:set_coordinates_from_panel()
        s.count = s.period
        return
    end
    s.count = int.u8(s.count - 1)
    if s.count == s.half then
        me:set_panel_from_coordinates()
    end
end

-- 0x080D1F10: its wait; then the spin, with its sound.
local function wait(me: Object)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    me.action = SPIN
    battle.play_sound(SOUND_SPIN)
end

-- It goes: moved out of sight (its Y's whole part 240), its region off.
local function gone(me: Object)
    local p = me.pos
    me.pos = Vec3.new(p.x, bit32.bor(bit32.lshift(240, 16), bit32.band(p.y, 0xFFFF)), p.z)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080D1F26: it moves; over a panel that isn't solid (but on column 7) it goes in a puff, past the right edge it goes.
local function spin(me: Object)
    move(me)
    if me.panel_x ~= 7 and not field.check(me.panel_x, me.panel_y, SOLID, 0) then
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), PUFF)
        gone(me)
    elseif me.pos.x >= RIGHT_EDGE then
        gone(me)
    end
    me:update_collision_panels()
end

-- Its region ends, and it goes.
local function ends(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080D1E94: its hits resolve and spark; the battle over, it goes; a body hit, it goes; another hit ends its region
-- (or it, while it waits). Its action; a region again on another panel than the one it was hit on; presented while its
-- panel is valid, else it goes.
local function tick(me: Object)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        return ends(me)
    end
    local hit = c.hit_flags
    if bit32.btest(hit, ENDS) then
        return ends(me)
    end
    if bit32.btest(hit, STOPS) then
        if me.action == WAIT then
            return ends(me)
        end
        c:set_region(nil)
        s.hit_at = me.panel_x + me.panel_y * 0x100
    end
    local a = me.action
    if a == WAIT then
        wait(me)
    elseif a == SPIN then
        spin(me)
    else
        error(string.format("TopMan's top's action %#x reads past its table (0x080D1F08)", a))
    end
    if me.lifecycle ~= "destroy" and c.region == nil and me.panel_x + me.panel_y * 0x100 ~= s.hit_at then
        c:set_region(regions.single)
    end
    if field.valid(me.panel_x, me.panel_y) then
        c:present()
    else
        ends(me)
    end
end

-- 0x080D1DF4: on a valid, solid panel (else gone at once): TopMan's sprite in animation 8 with its shadow; its wait
-- (30 ticks, 15 more for each top before it), its speeds (0x080D1FA0: across, back for the second and third; along,
-- up for the third and fourth), facing its side's way, on its panel; a body of row 10 hitting navis (modifier 3, the
-- heavy spark), its region the panel; its sound.
local function init(me: Object, number: number)
    local s = me.state :: State
    if not field.valid(me.panel_x, me.panel_y) or not field.check(me.panel_x, me.panel_y, SOLID, 0) then
        me:free()
        return
    end
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 8, 8
    sprite:set_animation(8)
    me.timer = number * 15 + 30
    s.period, s.count, s.half = PERIOD, PERIOD, PERIOD // 2
    local vx = if number >= 1 and number <= 2 then -VX else VX
    local vy = if number >= 2 then -VY else VY
    me.vel = Vec3.new(vx * me.facing, vy, me.vel.z)
    sprite.palette = 0
    sprite:set_flip(me.alliance)
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown, collision.hits_navis, 3)
    me.collision:set_region(regions.single)
    me.collision:set_hit_effect(sparks.heavy)
    me.collision:present()
    battle.play_sound(SOUND_APPEAR)
    me:set_lifecycle("update")
    tick(me)
end

top.kind = new.kind {
    id = "topman/top",
    pool = "attack",
    state = { number = "u8", period = "u8", half = "u8", count = "u8", hit_at = "u16" },
    -- 0x080D1DD0: by its state, then its sprite, while dimmed too.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me, (me.state :: any).number :: number)
        elseif l == "update" then
            tick(me)
        else
            -- 0x08010560: its collision, then itself, freed.
            me.collision:free()
            me:free()
            return
        end
        if me.active then
            me:update_sprite_while_dimmed()
        end
    end,
}

-- 0x080D1F78: top `number` (0 to 3) of TopMan `navi`'s, on (x, y) with his element and damage word, of his side,
-- running while dimmed. (Its position is the spawner's registers: the row, the element and his height, of which its init
-- keeps the height.)
function top.spawn(navi: Object, x: number, y: number, number: number): Object?
    local o = battle.spawn(top.kind, Vec3.new(y, navi.element, navi.pos.z))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = navi.element
    o:set_damage_word(bit32.bor(navi.damage, bit32.lshift(navi.stamina, 16)))
    o.alliance = navi.alliance
    o.run_while_dimmed = true
    ;(o.state :: any).number = number
    return o
end

return top

--!strict
-- EXE4's towers (attack object 0x92, 0x080DF68C: EXE4's own), which the towers' controller (lib/towers) raises:
-- AquaUp's aqua tower (row 0) and GreenWd's wood tower (row 1). On its panel, unturned, a thrown body hitting navis
-- once (its region the collision's single panel, gone after its first hit), it rises with its sound, holds and sinks
-- in its row's animations and ticks, then goes. Its rows are 0x080DF674's (12 bytes: the sprite, three animations
-- and their ticks, 0xFF for none, the hit modifier and the sound).

local collision = require("../rules/collision")
local sparks = require("../lib/sparks")

local tower = {}

-- A row: the sprite, then each step's animation (nil: the one before goes on) and ticks (a step lasts its ticks and
-- one more), the hit modifier and the sound as it rises.
export type TowerRow = {
    sprite: SpriteAsset,
    steps: { { anim: number?, ticks: number } },
    hit_mod: number,
    sound: Sound,
}

type State = { row: TowerRow }

local function row(r: TowerRow): TowerRow
    return new.record("tower-row", r)
end

tower.rows = {
    -- Row 0: 0c-25, animations 0, 1 and 2 for 12, 4 and 9 ticks; hit modifier 3; sound 0x120.
    aqua = row {
        sprite = asset.sprite("geyser"),
        steps = { { anim = 0, ticks = 0x0B }, { anim = 1, ticks = 3 }, { anim = 2, ticks = 8 } },
        hit_mod = 3,
        sound = asset.sound("aqua-tower"),
    },
    -- Row 1: 0c-0b, animation 0 for 12 ticks and 4 more, then 1 for 10; hit modifier 1; sound 0x13E.
    wood = row {
        sprite = asset.sprite("wood-tower"),
        steps = { { anim = 0, ticks = 0x0B }, { ticks = 3 }, { anim = 1, ticks = 9 } },
        hit_mod = 1,
        sound = asset.sound("wood-tower"),
    },
}

local RISE, HOLD, SINK = 0x0, 0x4, 0x8

-- 0x080DF754, 0x080DF77C, 0x080DF7A6: a step of the row: on its first tick its ticks, and the sound rising (its
-- animation the init's) or the step's animation after; past them the next step, past the last gone.
local function step(me: Object, row: TowerRow, i: number)
    local st = row.steps[i]
    if me.phase_init == 0 then
        if i == 1 then
            battle.play_sound(row.sound)
        elseif st.anim then
            me.anim = st.anim
        end
        me.timer = st.ticks
        me.phase_init = 4
    end
    local t = me.timer - 1
    if t >= 0 then
        me.timer = t
        return
    end
    me.timer = 0xFFFF
    if i < #row.steps then
        me.action = i * 4
        me.phase, me.phase_init = 0, 0
    else
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end
end

-- 0x080DF71A: the hits against it resolve and spark (its region gone after it hits), the step, present.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local row = (me.state :: State).row
    local a = me.action
    if a == RISE then
        step(me, row, 1)
    elseif a == HOLD then
        step(me, row, 2)
    elseif a == SINK then
        step(me, row, 3)
    else
        error(string.format("a tower's action %#x reads past its table", a))
    end
    c:present()
end

-- 0x080DF6B0: its row's sprite (with its shadow drawn with it) on its panel, its first animation requested (the
-- loaded one 0xFF, which the init's own load and step take: the update's sprite step loads the animation); a thrown
-- body hitting navis with the row's modifier and the element's spark; then its first tick.
local function init(me: Object)
    local row = (me.state :: State).row
    local sprite = me.sprite
    sprite:load(row.sprite)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = row.steps[1].anim :: number, 0xFF
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown, collision.hits_navis, row.hit_mod)
    me.collision:set_hit_effect(sparks.of_element(me.element))
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

tower.kind = new.kind {
    id = "tower",
    pool = "attack",
    state = { row = "record:tower-row" },
    -- 0x080DF68C: then the sprite, even while dimmed (0x08014446).
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080DF7D0: a tower of `row` on (x, y) with the element and damage word, of `owner`'s side. (Its position is the
-- registers until its init places it.)
function tower.spawn(owner: Object, x: number, y: number, row: TowerRow, element: number, damage: number)
    local o = battle.spawn(tower.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance = owner.alliance
        o:set_damage_word(damage)
        local s = o.state :: State
        s.row = row
    end
end

return tower

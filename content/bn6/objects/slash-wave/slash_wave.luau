--!strict
-- A sword wave (attack object #0x62, `sub_80D07CC`): 20 pixels up over its
-- panel it winds up (Param1 0: with a hit on its panel at once and 10
-- ticks later), then swings (its region on) and flies ahead, gone off the
-- field or once it hits an enemy's body. SlashMan's (Param1 1) wind up
-- longer, act and step their sprite while dimmed, and if the user holds B
-- while one swings it steers toward the nearest row with an enemy navi;
-- Param1 0 (his navi AI's) waits a dimming out. See docs/engine/chips.md
-- §3.6.18.

local slash_wave = {}

local SPRITE = "10-39"

-- Its height (a halfword store over Z's whole part).
local HEIGHT = 0x14

-- By Param1: its wind-up (`byte_80D0914`), its swing (`byte_80D0950`) and
-- its speed (`byte_80D0A54`, 16.16).
local WIND_UPS = { [0] = 0x14, 0x28 }
local SWINGS = { [0] = 0xA, 0xF }
local SPEEDS = { [0] = 0x6_0000, 0xC_0000 }

-- What ends it when hit (`byte_80D08C4`), by side: any bit of the word.
local ENDS = { [0] = 0x0580_0000, 0x0A80_0000 }

-- What it steers toward (`off_80D0A3C`): an enemy navi's body, by side;
-- the rows it tries, nearest first (`byte_80D0A48`).
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }
local ROWS = { 0, -1, 1, -2, 2 }

-- `sub_80D0996`: a wave on (x, y) for `owner` (the navi chip's user) with
-- `element`, the damage word and `params` {Param1}. `z` is the spawner's
-- register that becomes its Z, whose fraction it keeps.
function slash_wave.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    params: { number },
    damage: number,
    z: number
): Object?
    local o = battle.spawn_kind("slash-wave", Vec3.new(y, element, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.related1 = owner
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

local function table_at<T>(t: { [number]: T }, i: number, what: string): T
    local v = t[i]
    if v == nil then
        error(string.format("sword wave %d reads past %s", i, what))
    end
    return v
end

-- `sub_80D07FC`: over its panel; collision on, region off.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.bor(bit32.band(p.z, 0xFFFF), HEIGHT * 0x10000))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(4, 5, 3)
    local c = me.collision
    c.hit_effect = 0xFF
    c.region = 0
    c:present()
    me:set_param(2, 0)
    me:set_lifecycle("update")
end

-- `sub_80D0A5C`: Param1 0's hit on its panel.
local function hit(me: Object)
    if me:param(1) ~= 0 then
        return
    end
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = 1,
        hit_effect = 0xFF,
        target = 5,
        self_type = 4,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 1,
    })
end

-- `sub_80D08CC`: the wind-up (sound 0xB7), hitting at its start and 10
-- ticks in (Param1 0).
local function wind_up(me: Object)
    if me.phase == 0 then
        me.phase = 4
        hit(me)
        battle.play_sound(0xB7)
        local ticks = table_at(WIND_UPS, me:param(1), "byte_80D0914")
        me.timer, me.timer2 = ticks, ticks
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == me.timer2 - 0xA then
        hit(me)
        return
    end
    if left <= 0 then
        me:set_action(4)
    end
end

-- `sub_80D0A80`: SlashMan's: the user holding B steers it (once; sound
-- 0x8B).
local function steer(me: Object)
    if me:param(1) == 0 or me:param(2) ~= 0 then
        return
    end
    if (me.related1 :: Object):held("b") then
        me:set_param(2, 1)
        battle.play_sound(0x8B)
    end
end

-- `sub_80D0916`: the swing, its region on.
local function swing(me: Object)
    steer(me)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 1
        me.collision.region = 1
        me.timer = table_at(SWINGS, me:param(1), "byte_80D0950")
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left < 0 then
        me:set_action(8)
    end
end

-- `object_getFirstPanelInDirectionFiltered`: from (x, y) along `step`, the
-- first column whose panel passes `field.check`, while on the field; 0 if
-- none.
local function first_in_direction(x: number, y: number, step: number, require: number, forbid: number): number
    while true do
        if field.check(x, y, require, forbid) then
            return x
        end
        x += step
        if not field.valid(x, y) then
            return 0
        end
    end
end

-- `sub_80D09BE`: its speed ahead; steered, down or up toward the nearest
-- row with an enemy navi ahead, arriving as it reaches that column (two
-- BIOS divisions).
local function aim(me: Object)
    local speed = table_at(SPEEDS, me:param(1), "byte_80D0A54")
    local vy = 0
    if me:param(2) ~= 0 then
        local front = me.facing
        for _, dy in ROWS do
            local found = first_in_direction(me.panel_x + front, me.panel_y + dy, front, ENEMY_NAVI[me.alliance], 0)
            if found ~= 0 then
                local ticks = int.tdiv(int.mul32(math.abs(found - me.panel_x), 0x28_0000), speed)
                vy = int.tdiv(int.mul32(0x18_0000, dy), ticks)
                break
            end
        end
    end
    me.vel = Vec3.new(int.mul32(me.facing, speed), vy, me.vel.z)
end

-- `sub_80D0952`: fly (sound 0xB3) until off the field.
local function fly(me: Object)
    if me.phase == 0 then
        me.phase = 4
        battle.play_sound(0xB3)
        aim(me)
        return
    end
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(p.x + v.x, p.y + v.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if not field.valid(me.panel_x, me.panel_y) then
        me.collision.region = 0
        me.lifecycle = "destroy"
    end
end

local ACTIONS = { [0] = wind_up, [4] = swing, [8] = fly }

-- `sub_80D0868`: resolve the hits; gone once the battle is over or it hit
-- an enemy's body; act (outside a dimming unless it is SlashMan's) and
-- register again.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() or bit32.band(c.hit_flags, ENDS[me.alliance]) ~= 0 then
        c.region = 0
        me.lifecycle = "destroy"
        return
    end
    if me:param(1) == 0 and battle.dimmed() then
        return
    end
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("sword wave action %#x reads past its table (off_80D08B4)", me.action))
    end
    action(me)
    c:present()
end

-- `sub_80D07CC`.
function slash_wave.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    if me:param(1) == 0 then
        me:update_sprite_while_paused()
    else
        me:update_sprite_while_dimmed()
    end
end

return slash_wave

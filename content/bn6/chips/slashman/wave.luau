--!strict
-- A sword wave (the original's attack object #0x62, `sub_80D07CC`): 20
-- pixels up over its panel it winds up, then swings (its region on) and
-- flies ahead, gone off the field or once it hits an enemy's body. What it
-- is is its variant (a `SlashWave`, types.d.luau; the original's Param1):
-- the navi chip's (1) winds up longer, acts and steps its sprite while
-- dimmed, and if the user holds B while it swings it steers toward the
-- nearest row with an enemy navi; SlashMan's own AI's (0) hits its panel
-- as it starts to wind up and 10 ticks later, and waits a dimming out. See
-- docs/engine/chips.md §3.6.18.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local wave = {}

export type State = {
    -- What it is.
    variant: SlashWave,
    -- The user steered it (Param2).
    steered: boolean,
}

local SPRITE = asset.sprite("bn6:slash-wave")
local SOUND_AQUA_NEEDLE_2 = asset.sound("bn6:aqua-needle-2")
local SOUND_BOOMERANG = asset.sound("bn6:boomerang")
local SOUND_OK_8B = asset.sound("bn6:ok-8b")

-- Its height (a halfword store over Z's whole part).
local HEIGHT = 0x14
-- Its own hit's modifier, and its wind-up hits'.
local HIT_MOD, EARLY_HIT_MOD = 3, 1

-- The variants (the original's rows of `byte_80D0914`, the wind-up,
-- `byte_80D0950`, the swing, and `byte_80D0A54`, the speed, 16.16):
-- SlashMan's own AI's, and the navi chip's.
wave.plain = define.record("slashman-wave", {
    wind_up = 0x14,
    swing = 0xA,
    speed = 0x6_0000,
    hits_early = true,
} :: SlashWave)
wave.chip = define.record("slashman-wave", {
    wind_up = 0x28,
    swing = 0xF,
    speed = 0xC_0000,
    steers = true,
    while_dimmed = true,
} :: SlashWave)

-- Its actions.
local WIND_UP, SWING, FLY = 0, 4, 8

-- What ends it when hit (`byte_80D08C4`), by side: any bit of the word.
local ENDS = { [0] = 0x0580_0000, 0x0A80_0000 }

-- What it steers toward (`off_80D0A3C`): an enemy navi's body, by side;
-- the rows it tries, nearest first (`byte_80D0A48`).
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }
local ROWS = { 0, -1, 1, -2, 2 }

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
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    local c = me.collision
    c:set_hit_effect(nil)
    c:set_region(nil)
    c:present()
    local s = me.state :: State
    s.steered = false
    me:set_lifecycle("update")
end

-- `sub_80D0A5C`: the hit on its panel as it winds up (not the navi
-- chip's).
local function hit(me: Object)
    if not (me.state :: State).variant.hits_early then
        return
    end
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = regions.single,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = EARLY_HIT_MOD,
    })
end

-- `sub_80D08CC`: the wind-up, hitting at its start and 10 ticks in.
local function wind_up(me: Object)
    if me.phase == 0 then
        me.phase = 4
        hit(me)
        battle.play_sound(SOUND_BOOMERANG)
        local ticks = (me.state :: State).variant.wind_up
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
        me:set_action(SWING)
    end
end

-- `sub_80D0A80`: the navi chip's: the user holding B steers it (once).
local function steer(me: Object)
    local s = me.state :: State
    if not s.variant.steers or s.steered then
        return
    end
    if (me.related1 :: Object):held("b") then
        s.steered = true
        battle.play_sound(SOUND_OK_8B)
    end
end

-- `sub_80D0916`: the swing, its region on.
local function swing(me: Object)
    steer(me)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 1
        me.collision:set_region(regions.single)
        me.timer = (me.state :: State).variant.swing
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left < 0 then
        me:set_action(FLY)
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
    local s = me.state :: State
    local speed = s.variant.speed
    local vy = 0
    if s.steered then
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

-- `sub_80D0952`: fly until off the field.
local function fly(me: Object)
    if me.phase == 0 then
        me.phase = 4
        battle.play_sound(SOUND_AQUA_NEEDLE_2)
        aim(me)
        return
    end
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(p.x + v.x, p.y + v.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if not field.valid(me.panel_x, me.panel_y) then
        me.collision:set_region(nil)
        me.lifecycle = "destroy"
    end
end

local ACTIONS = { [WIND_UP] = wind_up, [SWING] = swing, [FLY] = fly }

-- `sub_80D0868`: resolve the hits; gone once the battle is over or it hit
-- an enemy's body; act (outside a dimming unless it is the navi chip's)
-- and register again.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() or bit32.band(c.hit_flags, ENDS[me.alliance]) ~= 0 then
        c:set_region(nil)
        me.lifecycle = "destroy"
        return
    end
    if not (me.state :: State).variant.while_dimmed and battle.dimmed() then
        return
    end
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("sword wave action %#x reads past its table (off_80D08B4)", me.action))
    end
    action(me)
    c:present()
end

wave.kind = define.kind {
    id = "bn6:slashman/wave",
    pool = "attack",
    state = { variant = "record:slashman-wave", steered = "bool" },
    -- `sub_80D07CC`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        if (me.state :: State).variant.while_dimmed then
            me:update_sprite_while_dimmed()
        else
            me:update_sprite_while_paused()
        end
    end,
}

-- `sub_80D0996`: a wave on (x, y) for `owner` (the navi chip's user) with
-- `element`, the damage word and its variant. `z` is the spawner's
-- register that becomes its Z, whose fraction it keeps.
function wave.spawn(owner: Object, x: number, y: number, element: number, variant: SlashWave, damage: number, z: number): Object?
    local o = battle.spawn(wave.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.related1 = owner
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
        local s = o.state :: State
        s.variant = variant
    end
    return o
end

return wave

--!strict
-- RedSun's sun (EXE4's attack object #0x84, 0x080DDB34; EXE4's own): it rises over its user's panel, 70 pixels up,
-- and shines; throws five meteors (./meteor) at the panel three columns ahead, 16 ticks apart; then dives onto that
-- panel in 10 ticks, hitting it with the chip's damage, cracking the panels around it, with a shake, a burst and
-- sparkles. Then it goes and its controller (../../lib/sun_and_moon) moves on.

local collision = require("../../rules/collision")
local meteor = require("./meteor")
local regions = require("@exelib/regions")
local sparkles = require("../../objects/sparkles")
local sun_and_moon = require("../../lib/sun_and_moon")

local sun = {}

export type State = {
    -- The controller waiting for it (its +0x4C: the controller's flag).
    controller: Object?,
}

local SPRITE = asset.sprite("red-sun")
local SOUND_APPEAR = asset.sound("tops")
local SOUND_SHINE = asset.sound("sun-moon-shine")
local SOUND_DIVE = asset.sound("red-sun-dive")
-- Its burst where it lands (effect row 6) and its sparkles (their parameters 0x10000: 32 ticks, the sparkles' row 0,
-- of explosions, effect row 0, without their sound).
local BURST = new.effect { sprite = asset.sprite("burst"), anim = 0, palette = 0 }
local SPARKLE = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }
local SPARKLE_TICKS = 0x20

-- How high it is spawned (16.16: the controller's register, 70 pixels).
sun.HEIGHT = 70 * 0x1_0000

-- Its meteors: five, the first 31 ticks after it shines, 16 ticks apart; each of 40 damage with the counter byte
-- 0x85 (0x080DDC48), of variant 0 (./meteor's `fire`).
local METEORS = 5
local METEOR_DAMAGE = 0x0085_0028
-- The dive: 10 ticks onto the panel three columns toward the enemy.
local DIVE_TICKS = 10
-- Its landing's hit: its panel (the region word 0x1505FF01: one panel, no spark, a thrown break hitting navis), hit
-- modifier 3.
local HIT_MOD = 3
-- The landing cracks the 3x3 panels around it (0x080DDD84's offsets: by column, then row).
local AROUND = { { -1, -1 }, { -1, 0 }, { -1, 1 }, { 0, -1 }, { 0, 0 }, { 0, 1 }, { 1, -1 }, { 1, 0 }, { 1, 1 } }

-- Count the timer down: true once it fell below 0.
local function ran_out(me: Object): boolean
    local t = me.timer - 1
    me.timer = int.u16(t)
    return t < 0
end

-- Action 0 (0x080DDBBC): 45 ticks shining (its sound 28 ticks in); then animation 1 and action 4.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.timer = 44
        me.phase_init = 4
    end
    if me.timer == 17 then
        battle.play_sound(SOUND_SHINE)
    end
    if ran_out(me) then
        me.anim = 1
        me:set_action(4)
    end
end

-- Action 4 (0x080DDBF0), by phase: 31 ticks (0x080DDC10); the meteors (0x080DDC34); 61 ticks (0x080DDC6E), then
-- action 8.
local function meteors(me: Object)
    local phase = me.phase
    if phase == 0 then
        if me.phase_init == 0 then
            me.timer = 0x1E
            me.phase_init = 4
        end
        if ran_out(me) then
            me.timer2 = METEORS
            me.timer = 0
            -- (The phase byte alone: its init byte stays set, which the next phase doesn't read.)
            me.phase = 4
        end
    elseif phase == 4 then
        if not ran_out(me) then
            return
        end
        me.timer = 0xF
        -- (Its element is the register the object loop leaves: 0 in every recording.)
        local o = meteor.spawn(me, me.panel_x, me.panel_y, 0, meteor.fire, METEOR_DAMAGE, me.pos.z)
        if o then
            o.run_while_dimmed = true
        end
        local left = me.timer2 - 1
        me.timer2 = int.u16(left)
        if left <= 0 then
            me.phase, me.phase_init = 8, 0
        end
    elseif phase == 8 then
        if me.phase_init == 0 then
            me.timer = 0x3C
            me.phase_init = 4
        end
        if ran_out(me) then
            me:set_action(8)
        end
    else
        error(string.format("RedSun's sun: phase %#x reads past its table (0x080DDC04)", phase))
    end
end

-- Action 8 (0x080DDC90), by phase: aimed (0x080DDCB0); the dive (0x080DDCF2); 61 ticks, then the controller goes on
-- and it goes (0x080DDD96).
local function dive(me: Object)
    local phase = me.phase
    if phase == 0 then
        me.anim = 2
        me.timer = DIVE_TICKS
        local tx = field.center(me.panel_x + me.facing * 3, me.panel_y)
        local p = me.pos
        me.vel = Vec3.new(int.tdiv(int.i32(tx - p.x), DIVE_TICKS), me.vel.y, int.tdiv(int.i32(-p.z), DIVE_TICKS))
        me.timer = DIVE_TICKS
        battle.play_sound(SOUND_DIVE)
        me.phase, me.phase_init = 4, 0
    elseif phase == 4 then
        if not ran_out(me) then
            local p, v = me.pos, me.vel
            me.pos = Vec3.new(int.i32(p.x + v.x), p.y, int.i32(p.z + v.z))
            me:set_panel_from_coordinates()
            return
        end
        local x, y = me.panel_x, me.panel_y
        if field.valid(x, y) then
            -- (`sub_80C53A6`, 0x080CD81E: a hit that resolves while dimmed.)
            local box = battle.hitbox(me, {
                panel_x = x,
                panel_y = y,
                element = me.element,
                region = regions.single,
                hit_effect = nil,
                target = collision.hits_navis,
                self_type = collision.thrown_break,
                damage = me.damage,
                stamina = me.stamina,
                hit_mod = HIT_MOD,
            })
            if box then
                box.run_while_dimmed = true
            end
            for _, d in AROUND do
                field.crack(x + d[1], y + d[2])
            end
            battle.shake_camera(3, 0x3C)
            battle.effect(me.pos, BURST)
            sparkles.spawn(me, SPARKLE_TICKS, SPARKLE, true)
        end
        me.visible = false
        me.phase, me.phase_init = 8, 0
    elseif phase == 8 then
        if me.phase_init == 0 then
            me.timer = 0x3C
            me.phase_init = 4
        end
        if ran_out(me) then
            sun_and_moon.done((me.state :: State).controller)
            me:set_lifecycle("destroy")
        end
    else
        error(string.format("RedSun's sun: phase %#x reads past its table (0x080DDCA4)", phase))
    end
end

-- 0x080DDB9C: by action.
local function tick(me: Object)
    local a = me.action
    if a == 0 then
        appear(me)
    elseif a == 4 then
        meteors(me)
    elseif a == 8 then
        dive(me)
    else
        error(string.format("RedSun's sun: action %#x reads past its table (0x080DDBB0)", a))
    end
end

-- 0x080DDB58: its sprite (no shadow), from animation 0, turned to its side's way, no element, on its panel at the
-- height it was spawned at; its sound.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    me.visible = true
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.element = 0
    me:set_coordinates_from_panel()
    battle.play_sound(SOUND_APPEAR)
    me:set_lifecycle("update")
    tick(me)
end

local kind = new.kind {
    id = "redsun/sun",
    pool = "attack",
    state = { controller = "object" },
    -- 0x080DDB34.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080DDDBA: the sun on the controller's panel, `sun.HEIGHT` up (the spawner's register), with its element and the
-- damage word, of the controller's side alone; it acts while dimmed.
function sun.spawn(controller: Object, damage: number): Object?
    local o = battle.spawn(kind, Vec3.new(controller.panel_y, controller.element, sun.HEIGHT))
    if o then
        o.panel_x, o.panel_y = controller.panel_x, controller.panel_y
        o.element = controller.element
        o:set_damage_word(damage)
        o.alliance = controller.alliance
        ;(o.state :: State).controller = controller
        o.run_while_dimmed = true
    end
    return o
end

return sun

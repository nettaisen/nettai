--!strict
-- GutsMan as the GutPnch chips bring him (EXE4's actor object #0x42, 0x080C83F8: EXE4's own): on his user's panel, his
-- sprite (08-02) in his palette 0 (0x0800B950 by navi 2 and the chip's level 0); he appears (his animation 3, sound
-- 0xB0, 5 ticks), and on a solid panel stands a tick and punches (his animation 15, sound 0xA4): 8 ticks in, a hit on
-- the panel ahead (`sub_80C53A6`: it runs while dimmed); 20 ticks of the punch and 20 more, then he
-- leaves (his animation 4, 5 ticks) and his controller goes on. On a hole he leaves at once. His sprite steps while
-- dimmed (0x08014446). Each chip's controller brings him with its damage (./init).

local collision = require("../../rules/collision")
local controller = require("@exelib/navi_chips/controller")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local guts = {}

local SPRITE = asset.sprite("gutsman")
local APPEAR_SOUND = asset.sound("appear")
local PUNCH_SOUND = asset.sound("fist-swing")
-- His palette (0x0800B950: navi 2's palettes, 1, times the chip's level's, 0).
local PALETTE = 0
-- The animations: standing, appearing, leaving, punching.
local ANIM_STAND, ANIM_APPEAR, ANIM_LEAVE, ANIM_PUNCH = 0, 3, 4, 15
-- His hit: its modifier (0x21) and height: 0x080CD7E2 keeps none of the 16 pixels asked (r3), and the region object
-- starts at the registers, its Z the raw 16.
local HIT_MOD, HIT_Z = 0x21, 16
-- The punch's ticks, and the tick of them the hit comes on (the timer reading 12).
local PUNCH, HIT_AT, AFTER = 20, 12, 20

-- The actions.
local APPEARING, STANDING, PUNCHING, LEAVING = 0, 4, 8, 0xC

-- 0x080C8484: he appears; on a solid panel he stands, else he leaves.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.visible = true
        me.anim = ANIM_APPEAR
        battle.play_sound(APPEAR_SOUND)
        me.timer = 5
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    me:set_action(if field.solid(me.panel_x, me.panel_y) then STANDING else LEAVING)
end

-- 0x080C84C2: a tick standing.
local function stand(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_STAND
        me.timer = 0
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me:set_action(PUNCHING)
    end
end

-- 0x080C8526: the punch, its hit on the panel ahead as the timer reads 12.
local function punch(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_PUNCH
        battle.play_sound(PUNCH_SOUND)
        me.timer = PUNCH
        me.phase_init = 4
    end
    if me.timer == HIT_AT then
        local box = battle.hitbox(me, {
            panel_x = me.panel_x + panels.side_direction(me.alliance),
            panel_y = me.panel_y,
            element = 0,
            z = HIT_Z,
            region = regions.single,
            hit_effect = sparks.heavy,
            target = collision.hits_navis,
            self_type = collision.attack,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = HIT_MOD,
        })
        if box then
            box.run_while_dimmed = true
        end
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me.phase, me.phase_init = 8, 0
    end
end

-- 0x080C84E8 by its phase: a tick, the punch, 20 ticks more.
local function punching(me: Object)
    local p = me.phase
    if p == 0 then
        if me.phase_init == 0 then
            me.timer = 0
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            me.phase, me.phase_init = 4, 0
        end
    elseif p == 4 then
        punch(me)
    elseif p == 8 then
        if me.phase_init == 0 then
            me.timer = AFTER
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            me.anim = ANIM_STAND
            me:set_action(LEAVING)
        end
    else
        error(string.format("GutsMan's punch: phase %#x reads past 0x080C84FC", p))
    end
end

-- 0x080C859A: he leaves; his controller goes on.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_LEAVE
        me.timer = 5
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    local c = me.related1
    if c then
        controller.done(c)
    end
    -- (A byte store: his action and phase stay.)
    me.lifecycle = "destroy"
end

-- 0x080C8460.
local function tick(me: Object)
    local a = me.action
    if a == APPEARING then
        appear(me)
    elseif a == STANDING then
        stand(me)
    elseif a == PUNCHING then
        punching(me)
    elseif a == LEAVING then
        leave(me)
    else
        error(string.format("GutsMan's punch: action %#x reads past 0x080C8474", a))
    end
end

-- 0x080C841C: his look on his panel. (It stores 0xFF into his animation, not its copy, and loads that.)
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0xFF
    sprite:set_animation(0xFF)
    sprite.palette = PALETTE
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
    me:set_lifecycle("update")
end

guts.kind = new.kind {
    id = "gutpnch/guts",
    pool = "actor",
    -- 0x080C83F8: its state's routine, then its sprite, while dimmed too.
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
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080C85C2: GutsMan on the controller's panel, of its side and element, with the damage word; his controller waits
-- for him. (It is spawned at the spawner's registers; its init places it.)
function guts.spawn(c: Object, _user: Object, damage: number): Object?
    local o = battle.spawn(guts.kind, Vec3.zero)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = c.panel_x, c.panel_y
    o.element = c.element
    o.alliance = c.alliance
    o.related1 = c
    o:set_damage_word(damage)
    return o
end

return guts

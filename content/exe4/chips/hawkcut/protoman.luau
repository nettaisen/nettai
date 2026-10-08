--!strict
-- ProtoMan as the HawkCut chips bring him (EXE4's actor object #0x44, 0x080C8A28: EXE4's own, on the template of
-- GutPnch's GutsMan): on his user's panel, his sprite (08-07) in his palette 0 (0x0800B950 by navi 7 and the chip's
-- level 0); he appears (his animation 3, sound 0xB0, 5 ticks) and on a solid panel stands a tick and slashes twice
-- with his sword (attachment row 3, its animation 7, animating while dimmed), each slash his animation 5 from its
-- first frame and the swing's sound (0x8E): as its timer reads 12, a hit on the panel ahead (`sub_80C53A6`: running
-- while dimmed) and its effect there, 16 pixels up, facing his way: first a wide one (region 4, the effect table's row
-- 0x17), then a long one (region 2, row 0x18), 21 ticks each; 21 ticks more, his sword away, then he leaves (his
-- animation 4, 5 ticks) and his controller goes on. On a hole he leaves at once. His sprite steps while dimmed
-- (0x08014446).

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local controller = require("@exelib/navi_chips/controller")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")

local protoman = {}

export type State = {
    -- His sword (+0x60).
    held: Object?,
}

local SPRITE = asset.sprite("protoman")
local APPEAR_SOUND = asset.sound("appear")
local SWING_SOUND = asset.sound("sword-swing")
-- His palette (0x0800B950: navi 7's palettes, 1, times the chip's level's, 0).
local PALETTE = 0
local SWORD = attachment.look { sprite = asset.sprite("sword"), palette = 0 }
local SWORD_ANIM = 7
-- The effect table's rows 0x17 and 0x18: the slash (0c-14) across and along.
local WIDE_SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 }
local LONG_SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 1, palette = 0 }
-- The animations: standing, appearing, leaving, slashing.
local ANIM_STAND, ANIM_APPEAR, ANIM_LEAVE, ANIM_SLASH = 0, 3, 4, 5
-- A slash's ticks, and the tick of them its hit comes on (the timer reading 12); its effect's height.
local SLASH, HIT_AT, AFTER, EFFECT_Z = 20, 12, 20, 0x10_0000
local HIT_MOD = 3

-- The actions.
local APPEARING, STANDING, SLASHING, LEAVING = 0, 4, 8, 0xC

-- Count the timer down: true once it fell below 0.
local function ran_out(me: Object): boolean
    local t = me.timer - 1
    me.timer = int.u16(t)
    return t < 0
end

-- 0x080C8AB4: he appears; on a solid panel he stands, else he leaves.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.visible = true
        me.anim = ANIM_APPEAR
        me.timer = 5
        battle.play_sound(APPEAR_SOUND)
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t > 0 then
        return
    end
    me:set_action(if field.solid(me.panel_x, me.panel_y) then STANDING else LEAVING)
end

-- 0x080C8AF2: a tick standing.
local function stand(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_STAND
        me.timer = 0
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t <= 0 then
        me:set_action(SLASHING)
    end
end

-- A slash's hit on the panel ahead, of `region`, and its `effect` there.
local function strike(me: Object, region: Region, effect: Effect)
    local x, y = me.panel_x + panels.side_direction(me.alliance), me.panel_y
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = 0,
        region = region,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.slash,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
    if box then
        box.run_while_dimmed = true
    end
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, EFFECT_Z), effect, bit32.bxor(me.alliance, me.flip))
end

-- 0x080C8B5A and 0x080C8BD8: a slash (the sword drawn for the first, its animation restarted for the second).
local function slash(me: Object, first: boolean, region: Region, effect: Effect, next: number)
    if me.phase_init == 0 then
        me.anim, me.anim_loaded = ANIM_SLASH, 0xFF
        if first then
            attachment.attach(me, SWORD, "held", SWORD_ANIM, true)
        else
            local sword = (me.state :: State).held
            if sword then
                sword.anim_loaded = 0xFF
            end
        end
        me.timer = SLASH
        battle.play_sound(SWING_SOUND)
        me.phase_init = 4
    end
    if me.timer == HIT_AT then
        strike(me, region, effect)
    end
    if ran_out(me) then
        me.phase, me.phase_init = next, 0
    end
end

-- 0x080C8B18 by its phase: a tick, the wide slash, the long one, 20 ticks more and his sword away.
local function slashing(me: Object)
    local p = me.phase
    if p == 0 then
        -- 0x080C8B3C.
        if me.phase_init == 0 then
            me.timer = 0
            me.phase_init = 4
        end
        if ran_out(me) then
            me.phase, me.phase_init = 4, 0
        end
    elseif p == 4 then
        slash(me, true, regions.wide, WIDE_SLASH, 8)
    elseif p == 8 then
        slash(me, false, regions.long, LONG_SLASH, 0xC)
    elseif p == 0xC then
        -- 0x080C8C4C.
        if me.phase_init == 0 then
            me.timer = AFTER
            me.phase_init = 4
        end
        if ran_out(me) then
            (me.state :: State).held = nil
            me.anim = ANIM_STAND
            me:set_action(LEAVING)
        end
    else
        error(string.format("HawkCut's ProtoMan: phase %#x reads past 0x080C8B2C", p))
    end
end

-- 0x080C8C76: he leaves; his controller goes on.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_LEAVE
        me.timer = 5
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
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

-- 0x080C8A90.
local function tick(me: Object)
    local a = me.action
    if a == APPEARING then
        appear(me)
    elseif a == STANDING then
        stand(me)
    elseif a == SLASHING then
        slashing(me)
    elseif a == LEAVING then
        leave(me)
    else
        error(string.format("HawkCut's ProtoMan: action %#x reads past 0x080C8AA4", a))
    end
end

-- 0x080C8A4C: his look on his panel, on the ground. (It stores 0xFF into his animation, not its copy, and loads that.)
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

protoman.kind = new.kind {
    id = "hawkcut/protoman",
    pool = "actor",
    state = { held = "object" },
    -- 0x080C8A28: its state's routine, then its sprite, while dimmed too.
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

-- 0x080C8C9E: ProtoMan on the controller's panel, of its side and element, with the damage word; his controller waits
-- for him. (He is spawned at the spawner's registers; his init places him.)
function protoman.spawn(c: Object, _user: Object, damage: number): Object?
    local o = battle.spawn(protoman.kind, Vec3.zero)
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

return protoman

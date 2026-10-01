--!strict
-- A boulder (the original's attack object #0x6E, `sub_80D2290`): the field
-- obstacle some stages start with (the actor lists' entry type 3,
-- `sub_8007450`: the kind's `place`, the role `kinds.boulder`), 500 HP, on
-- its panel's side. It stands through the round's start and takes its
-- place once the fight is on; it has no lifetime. Broken, it leaves two
-- chunks of rock debris and a splash of dust; a chip can remove or absorb
-- it. It is one of the field's two stage objects (the field-object
-- registry's stage slots, which it leaves when it goes). See
-- docs/engine/field-objects.md §3.1.
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions.

local absorbed_obstacle = require("../absorbed-obstacle/absorbed_obstacle")
local collision = require("../../rules/collision")
local debris = require("../rock/debris")
local effects = require("../../lib/effects")

local boulder = {}

local SPRITE = asset.sprite("boulder")
-- How it looks absorbed (the original's obstacle kind 5: its own sprite).
local ABSORBED = absorbed_obstacle.look { sprite = SPRITE }
-- Its NameID (the field objects' look, a number until identities come).
local NAME = 0xCE
local HP = 500
-- What its timer starts at (nothing counts it down).
local TIMER = 0x1770
-- The damage it does when thrown.
local DAMAGE = 200
-- What it is and reaches: a neutral body that everything reaches, while
-- dimmed too (rows 0x0E and 0x0F); hit modifier 3.
local IS, HITS, HIT_MOD = collision.neutral_body, collision.everything, 3
-- The palette of the debris it breaks into.
local DEBRIS_PALETTE = 0
-- A panel of side 1's.
local SIDE_1 = 0x20

-- Its actions (`jt_80D2348`).
local WAIT, DESTROYED, STANDING = 0, 2, 8

-- `sub_80D236C`: once the fight is on, it stands, and no longer runs while
-- the battle is paused.
local function wait(me: Object)
    if not battle.fighting() then
        return
    end
    me.run_while_paused = false
    me.visible = true
    me:set_action(STANDING)
end

-- `sub_80D2398`.
local function standing(_me: Object) end

-- `sub_80D239C`: leave the field and its stage slot: absorbed, removed by a
-- chip (a puff, or blinking out), or broken into two chunks and dust.
local function destroyed(me: Object)
    obstacle.release_tracking(me)
    me.collision:set_region(nil)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    obstacle.leave_stage(me)
    local how = obstacle.removal(me)
    if how == "absorbed" then
        obstacle.fly_to_absorber(me, ABSORBED)
    elseif how ~= "broken" then
        local blink = obstacle.blink_out(me)
        if blink == "blinking" then
            return
        elseif blink == "no" then
            local p = me.pos
            battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), effects.puff)
        end
    else
        local p = me.pos
        for _ = 1, 2 do
            debris.spawn(battle.jitter(0xF, p), DEBRIS_PALETTE)
        end
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), effects.splash)
    end
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- The boulder's action table (`jt_80D2348`): its own actions and the
-- framework's shared ones.
local ACTIONS: { [number]: (Object) -> () } = {
    [WAIT] = wait,
    [1] = obstacle.return_to_idle,
    [DESTROYED] = destroyed,
    [3] = obstacle.flinch,
    [4] = obstacle.paralyzed,
    [5] = obstacle.slide,
    [6] = obstacle.frozen,
    [7] = obstacle.bubbled,
    [STANDING] = standing,
}

-- `sub_80D2320`: hits, the reaction, its action; no hit spark and no
-- lifetime. Its sprite steps while dimmed too.
local function tick(me: Object)
    me.visible = true
    obstacle.take_hits(me)
    local a = obstacle.react(me)
    if a then
        local action = ACTIONS[a]
        if not action then
            error(string.format("boulder action %d reads past its table", a))
        end
        action(me)
    end
    me.collision:present()
    me:update_sprite_while_dimmed()
end

-- `sub_80D22B0`: on its panel, a pixel back and a pixel down (so what
-- stands on the panel's center is drawn over it), with its collision on
-- the panel at once. Its first update is the next tick's.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:step()
    sprite.shadow = "ground"
    me:set_coordinates_from_panel()
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y - 0x1_0000, -0x1_0000)
    me.name_id = NAME
    me.hp, me.max_hp = HP, HP
    me.timer = TIMER
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(IS, HITS, HIT_MOD)
    me.collision:present()
    me.lifecycle = "update"
end

-- The kind (its `place` spawns one).
local KIND: Kind
KIND = define.kind {
    id = "boulder",
    pool = "attack",
    -- `sub_80D2290`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
    -- `sub_8007450`: a stage's boulder (an actor list's entry type 3), on
    -- its panel's side, if one of the field's two stage slots is free;
    -- running while the battle is paused or dimmed. (`sub_80D2430`; until
    -- its init places it, its position is the spawner's registers: Y and
    -- what nothing reads.)
    place = function(spec: ActorListEntrySpec): Object?
        if not obstacle.stage_slot_free() then
            return nil
        end
        local x, y = spec.panel_x, spec.panel_y
        local o = battle.spawn(KIND, Vec3.new(y, 0, 0))
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = x, y
        o:set_damage_word(DAMAGE)
        o.element = 0
        o.alliance = if bit32.btest(field.flags(x, y), SIDE_1) then 1 else 0
        o.run_while_paused = true
        o.run_while_dimmed = true
        obstacle.enter_stage(o)
        return o
    end,
}
boulder.kind = KIND

return boulder

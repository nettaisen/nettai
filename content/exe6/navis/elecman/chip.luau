--!strict
-- ElecMan's own chip, DElecSwd (the original's `sub_80F09B8`, his action
-- table's entry 0xA): glowing (a follow effect on him), he slides forward
-- at 5 pixels a tick, checking at each panel's center whether to stop: a
-- body or a neutral object on the next panel, the field's end after it, or
-- an enemy navi in reach of a wide slash. There he slashes the panel ahead
-- (top to bottom) and comes back to his panel. See
-- docs/engine/standard-chips.md, "Action 0x0A".

local collision = require("../../rules/collision")
local follow_effect = require("../../objects/follow_effect")
local link = require("../../lib/link_chips")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local statuses = require("../../rules/status")
local swords = require("../../lib/swords/parts")

export type State = {
    timer: number,
    -- His glow (AIAttackVars+0x30).
    glow: Object?,
}

local STATE = { timer = "u16", glow = "object" }

local SLIDE, SLASH, RETURN, RECOVER = 0x0, 0x4, 0x8, 0xC
local ANIM_SLIDE, ANIM_SLASH, ANIM_RETURN = 0x10, 0x11, 3
local SPEED = 0x5_0000
local SOUND = asset.sound("sword-swing")

-- His glow: the follow effect's look 7, riding on him while he slides and
-- slashes; it shows the slash's animation once he slashes.
local GLOW = follow_effect.look { sprite = asset.sprite("elec-sword"), anim = 0xF, palette = 0, behavior = "riding" }
local GLOW_SLASH_ANIM = 0x10

-- What stops him (`sub_80F0BD2`): on the next panel, another body of
-- either side or a neutral object; after it, no field; or an enemy navi's
-- body on the wide slash's panels around the next (`off_80F0C24`, by side).
local BLOCKED, ON_FIELD = 0x0380_0000, 0x0001_0000
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }

-- The slash: a wide hit on the panel ahead (toward the enemy's side), elec
-- spark, paralyzing; its effect (the wide sword's slash, 16 pixels up,
-- palette 3) over the panel ahead (his facing's).
local SLASH_HIT = { is = collision.slash, hits = collision.hits_navis, hit_mod = 1, status = statuses.paralyze_90 }
local SLASH_Z, SLASH_PALETTE = 0x10_0000, 3

local SLASH_TICKS, RETURN_TICKS, RECOVER_TICKS = 0x14, 3, 0x1E

-- `sub_80F0BD2`: whether to stop and slash from here. The last check draws
-- the RNG once, when it finds an enemy.
local function stop_here(me: Object): boolean
    local x, y = me.panel_x + me.facing, me.panel_y
    if bit32.btest(field.flags(x, y), BLOCKED) then
        return true
    end
    if not bit32.btest(field.flags(x + me.facing, y), ON_FIELD) then
        return true
    end
    local found = panels.random_in(x, y, ENEMY_NAVI[me.alliance], 0, regions.wide, panels.side_direction(me.alliance))
    return found ~= nil
end

-- `sub_80F09DC`: the slide. Its entry sets him moving with his glow and
-- checks at once; each tick after he moves on, and checks when he reaches
-- or passes his panel's center. (The game tests the center against the
-- velocity where the old X belongs.)
local function slide(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        link.add_bonus(me)
        me.anim = ANIM_SLIDE
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        me:reserve_panel(me.panel_x, me.panel_y)
        me:set_status("moving", true)
        me:set_status("using_action", true)
        me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
        local cx, cy = field.center(me.panel_x, me.panel_y)
        s.glow = follow_effect.spawn(me, Vec3.new(cx, cy, 0), GLOW, me.alliance)
        if stop_here(me) then
            me:set_panel_from_coordinates()
            link.go(me, SLASH)
        end
        return
    end
    local cx = field.center(me.panel_x, me.panel_y)
    local vx = me.vel.x
    local x = int.i32(me.pos.x + vx)
    me.pos = Vec3.new(x, me.pos.y, me.pos.z)
    if cx ~= vx and link.between(x, vx, cx) and stop_here(me) then
        me:set_panel_from_coordinates()
        link.go(me, SLASH)
        return
    end
    me:set_panel_from_coordinates()
    me:update_collision_panels()
end

-- `sub_80F0B80`, `sub_80F0BA4`: the hit on the panel ahead toward the
-- enemy's side, and the slash's effect over the panel ahead.
local function strike(me: Object)
    battle.hitbox(me, {
        panel_x = me.panel_x + panels.side_direction(me.alliance),
        panel_y = me.panel_y,
        element = me.attack_element,
        z = 0,
        region = regions.wide,
        hit_effect = sparks.elec,
        target = SLASH_HIT.hits,
        self_type = SLASH_HIT.is,
        damage = link.damage(me),
        stamina = link.stamina(me),
        hit_mod = SLASH_HIT.hit_mod,
        status = SLASH_HIT.status,
    })
    local cx, cy = field.center(me.panel_x + me.facing, me.panel_y)
    battle.effect(Vec3.new(cx, cy, SLASH_Z), swords.effects.wide, link.flip(me), SLASH_PALETTE)
end

-- `sub_80F0A7A`: the slash; 20 ticks.
local function slash(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_SLASH
        s.timer = SLASH_TICKS
        strike(me)
        battle.play_sound(SOUND)
        local glow = s.glow
        if glow then
            glow.anim = GLOW_SLASH_ANIM
        end
    elseif link.run_out(s) then
        link.go(me, RETURN)
    end
end

-- `sub_80F0AB6`: back on his panel, the move done, his glow gone; 3 ticks.
local function return_home(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_RETURN
        me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
        me:unreserve_panel(me.panel_x, me.panel_y)
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        me:set_status("moving", false)
        me:set_status("move_complete", true)
        me:set_status("using_action", false)
        s.timer = RETURN_TICKS
        local glow = s.glow
        if glow then
            follow_effect.stop(glow)
        end
    elseif link.run_out(s) then
        link.go(me, RECOVER)
    end
end

-- `sub_80F0B0E`: 30 ticks, then idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 0
        s.timer = RECOVER_TICKS
    elseif link.run_out(s) then
        me:exit_attack()
    end
end

local PHASES: { [number]: (me: Object, s: State) -> () } = { [SLIDE] = slide, [SLASH] = slash, [RETURN] = return_home, [RECOVER] = recover }

local chip: Chip = {
    codes = { "E" },
    element = "elec",
    family = "elec",
    class = "special",
    rarity = 1,
    mb = 99,
    damage = { formula = "navi_level", base = 40, per_level = 20 },
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    slot_in_limit = 3,
    -- `sub_80F09B8` (`off_80F09CC`).
    action = new.action {
        id = "delecswd/action",
        state = STATE,
        update = function(me: Object, s: State)
            link.run(me, s, PHASES, "DElecSwd (sub_80F09B8)")
        end,
    },
}

return { delecswd = chip }

--!strict
-- The Guardian statue (attack object #0x7D, `sub_80D4C84`): a field
-- obstacle with 1 HP that stands for 6000 ticks. Broken by an attack from
-- one side only, it takes the other side's part: in a dimming of its own
-- (objects/guardian-strike, which the other player can't cut in on) it
-- strikes every panel the attacker's navi stands on. Broken otherwise (by a
-- body, by hits from both sides, by its time running out) it just
-- crumbles. The Guardian chip places one (objects/guardian, Param1 0, with
-- the user's side); stages can place one at the start (the actor lists'
-- entry type 9, Param1 1, with its panel's side).
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions.

local rising_bubble = require("../rising-bubble/rising_bubble")
local guardian_strike = require("../guardian-strike/guardian_strike")

export type State = {
    -- What hit it when it broke (the collision's hit flags; the object's
    -- +0x60): which side to strike back at.
    attackers: number,
}

local guardian_statue = {
    state = { attackers = "u32" },
}

local SPRITE = "0C-35"
local NAME = 0xE3
local LIFETIME = 0x1770
-- It takes its panel after 3 ticks.
local APPEAR_TICKS = 3
-- Where it can stand: a solid panel with nothing on it.
local SOLID, OCCUPIED = 0x10, 0x0F88_0080
-- Its collision: an obstacle (types 0xE and 0xF, hit modifier 3).
local SELF_TYPE, TARGET_TYPE, HIT_MOD = 0xE, 0xF, 3
-- The damage the stages' statues strike with.
local STAGE_DAMAGE = 200
local PLACE_SOUND = 0x94
-- The flash it arrives in (effect #0 look 0x15, 16 pixels up), what's left
-- when it crumbles or a chip removes it (look 0x14, 16 or 12 pixels up;
-- with a rising bubble) and the sound.
local ARRIVAL, CRUMBS = 0x15, 0x14
local CRUMBLE_SOUND = 0x90
-- Its kind among absorbed obstacles (`data.objects.absorbed_sprites`).
local ABSORBED_KIND = 9
-- The collision types of each side's hits (`sub_80D4FF6`): side 0's
-- attacks, objects and bodies are the bits 0xAA000000, side 1's
-- 0x55000000.
local SIDE0_HITS, SIDE1_HITS = 0xAA00_0000, 0x5500_0000
-- The strike (`byte_80D4EB4`, by the side it strikes for): every panel with
-- the enemy navi (whole-field region 0x85 for side 0, 0x84 for side 1),
-- with a spark look 0x45 on each (`sub_801BD3C`), a sound, and the statue
-- in its striking animation.
local STRIKE_REGION: { [number]: number } = { [0] = 0x85, [1] = 0x84 }
local STRIKE_HIT_EFFECT, STRIKE_TARGET, STRIKE_SELF, STRIKE_HIT_MOD = 0xFF, 5, 0x17, 3
local STRIKE_SPARK = 0x45
local STRIKE_SOUND = 0x12E
local STRIKE_ANIM = 1

-- Its actions (`off_80D4D60`).
local APPEAR, DESTROYED, STANDING, STRIKE_BACK = 0, 2, 8, 9

-- `sub_80D4FA6`: a statue on (x, y) with the spawn parameters `params` and
-- the damage word `damage`: with Param1 0, of `owner`'s side and facing;
-- else of the panel's side. In `alliance`'s field-object slots (class 1:
-- one a side), running while dimmed (and while paused for Param1 set).
-- (Until its init places it, its position is `pos`, the spawner's
-- registers.)
function guardian_statue.spawn(owner: Object?, x: number, y: number, params: { number }, damage: number, pos: Vec3): Object?
    local o = battle.spawn_kind("guardian-statue", pos, params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.damage = bit32.band(damage, 0xFFFF)
    o.stamina = bit32.rshift(damage, 16)
    if o:param(1) == 0 then
        if not owner then
            error("sub_80D4FA6: a Guardian statue with Param1 0 copies its spawner's side")
        end
        o.alliance, o.flip = owner.alliance, owner.flip
    else
        local panel = field.panel(x, y)
        if not panel then
            error(string.format("sub_80D4FA6 reads the side of panel (%d, %d), off the field", x, y))
        end
        o.alliance, o.flip = panel.alliance, 0
        o.run_while_paused = true
    end
    o.run_while_dimmed = true
    obstacle.register(o, o.alliance, 1)
    return o
end

-- `sub_800751C`: an actor list's statue, on its panel's side.
function guardian_statue.actor_list_entry(spec: ActorListEntrySpec): Object?
    local x, y = spec.panel_x, spec.panel_y
    local panel = field.panel(x, y)
    if not panel then
        error(string.format("an actor list places a Guardian statue off the field at (%d, %d)", x, y))
    end
    return guardian_statue.spawn(nil, x, y, { 1, 0, 0, 0 }, STAGE_DAMAGE, Vec3.new(y, panel.alliance, spec.variant))
end

-- `sub_80D4FF6`: the side that strikes back: the one opposite the only
-- side whose hits broke it (nil: both sides' or neither's).
local function avenging_side(hits: number): number?
    local by0, by1 = bit32.btest(hits, SIDE0_HITS), bit32.btest(hits, SIDE1_HITS)
    if by1 and not by0 then
        return 0
    elseif by0 and not by1 then
        return 1
    end
    return nil
end

-- `sub_801BD3C`: sparks `look` over hit region `region` from (x, y), the
-- offsets' x toward `side`'s front, at height `z` (below 0x80: the hit
-- region's panels on the field; from 0x80: the whole field's panels
-- meeting condition `region - 0x80`, from (6, 3) back to (1, 1), at height
-- 0).
local function region_sparks(x: number, y: number, look: number, region: number, side: number, z: number)
    local i = bit32.band(region, 0x7F)
    if bit32.btest(region, 0x80) then
        local cond = data.rules.field_regions[i]
        if not cond then
            error(string.format("sub_801BD3C: whole-field region %#x reads past byte_8019C34", 0x80 + i))
        end
        for py = 3, 1, -1 do
            for px = 6, 1, -1 do
                if field.check(px, py, cond.require, cond.forbid) then
                    local cx, cy = field.center(px, py)
                    battle.effect(Vec3.new(cx, cy, 0), look)
                end
            end
        end
        return
    end
    if i == 0 then
        return
    end
    local offsets = data.regions[i]
    if not offsets then
        error(string.format("sub_801BD3C: hit region %#x reads past PanelOffsetListsPointerTable", i))
    end
    local dir = 1 - 2 * side
    for _, d in offsets do
        local px, py = x + d[1] * dir, y + d[2]
        if field.valid(px, py) then
            local cx, cy = field.center(px, py)
            battle.effect(Vec3.new(cx, cy, z), look)
        end
    end
end

-- Gone: its collision off its panels, hidden, destroyed next update.
local function leave(me: Object)
    me.collision.region = 0
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80D4F70`: crumble: the dust, a bubble, the sound.
local function crumble(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), CRUMBS)
    rising_bubble.spawn(p, 0)
    battle.play_sound(CRUMBLE_SOUND)
    leave(me)
end

-- `sub_80D4EFC`: leave the field; broken by one side's attack (its HP
-- run out, Param2 clear), strike back for the other side instead.
local function destroyed(me: Object)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    local how = obstacle.removal(me)
    if how == "absorbed" then
        obstacle.fly_to_absorber(me, ABSORBED_KIND)
        leave(me)
        return
    elseif how ~= "broken" then
        local blink = obstacle.blink_out(me)
        if blink == "blinking" then
            return
        elseif blink == "done" then
            battle.play_sound(CRUMBLE_SOUND)
            leave(me)
            return
        end
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), CRUMBS)
        crumble(me)
        return
    end
    if me.hp == 0 and me:param(2) == 0 then
        local hits = me.collision.hit_flags
        local s = me.state :: State
        s.attackers = hits
        if hits ~= 0 and avenging_side(hits) then
            me.action, me.phase, me.phase_init = STRIKE_BACK, 0, 0
            return
        end
    end
    crumble(me)
end

-- `sub_80D4D88`: after 3 ticks, take the panel if it's free solid ground;
-- else crumble at once.
local function appear(me: Object)
    local t = me.timer2 - 1
    me.timer2 = t
    if t ~= 0 then
        return
    end
    if not field.check(me.panel_x, me.panel_y, SOLID, OCCUPIED) then
        destroyed(me)
        return
    end
    me.collision.region = 1
    battle.play_sound(PLACE_SOUND)
    me.action, me.phase, me.phase_init = STANDING, 0, 0
end

-- `sub_80D4DC8`.
local function standing(_me: Object) end

-- `sub_80D4DF0`: once the battle isn't dimmed, take the avenging side's
-- part and start its dimming (it can't be cut in on); if the battle is
-- over, crumble.
local function start_strike(me: Object)
    if battle.dimmed() then
        return
    end
    me.prevent_anim = 0
    me.collision.region = 0
    local s = me.state :: State
    local side = if battle.over() then nil else avenging_side(s.attackers)
    if not side then
        me.collision.region = 0
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), CRUMBS)
        me.visible = false
        me:set_lifecycle("destroy")
        return
    end
    me.alliance = side
    -- (The HUD: sub_801DACC(0x40).)
    local controller = guardian_strike.spawn(me)
    dimming.start(side, true, controller, me)
    me.phase = 4
end

-- `sub_80D4E64`: once its dimming's effect says so (Param3), strike.
local function strike(me: Object)
    if me:param(3) == 0 then
        return
    end
    local region = STRIKE_REGION[me.alliance]
    local hit = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = region,
        hit_effect = STRIKE_HIT_EFFECT,
        target = STRIKE_TARGET,
        self_type = STRIKE_SELF,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = STRIKE_HIT_MOD,
    })
    if hit then
        hit.run_while_dimmed = true
    end
    -- (The game passes the panel's Y and the element as sub_801BD3C's
    -- panel, and 3 as its height, which only a hit region's sparks use.)
    region_sparks(me.panel_y, me.element, STRIKE_SPARK, region, me.alliance, 3)
    battle.shake_camera(1, 0x3C)
    battle.play_sound(STRIKE_SOUND)
    me.anim = STRIKE_ANIM
    me.phase = 8
end

-- `sub_80D4EBC`: after the strike's animation, the dust.
local function after_strike(me: Object)
    if not bit32.btest(me.sprite.frame_flags, 0x80) then
        return
    end
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), CRUMBS)
    me.phase = 0xC
end

-- `sub_80D4EE0`: gone, once the battle isn't dimmed.
local function strike_done(me: Object)
    me.collision.region = 0
    me.visible = false
    if battle.dimmed() then
        return
    end
    me:set_lifecycle("destroy")
end

-- `sub_80D4DCC`: striking back, by phase.
local function strike_back(me: Object)
    local phase = me.phase
    if phase == 0 then
        start_strike(me)
    elseif phase == 4 then
        strike(me)
    elseif phase == 8 then
        after_strike(me)
    elseif phase == 0xC then
        strike_done(me)
    else
        error(string.format("the Guardian statue's strike phase %#x reads past its table", phase))
    end
end

-- The statue's action table (`off_80D4D60`): its own actions and the
-- framework's shared ones.
local ACTIONS: { [number]: (Object) -> () } = {
    [0] = appear,
    [1] = obstacle.return_to_idle,
    [2] = destroyed,
    [3] = obstacle.flinch,
    [4] = obstacle.paralyzed,
    [5] = obstacle.knocked_back,
    [6] = obstacle.frozen,
    [7] = obstacle.bubbled,
    [8] = standing,
    [9] = strike_back,
}

-- `sub_80D4D34`: hits (a push keeps the damage), the lifetime, the
-- reaction (a body's touch destroys it, its HP as it is), its action.
local function tick(me: Object)
    me.visible = true
    me.sprite.white = false
    obstacle.take_hits(me, "keeps_damage")
    obstacle.tick_lifetime(me)
    local a = obstacle.react(me, "destroys")
    if a then
        local action = ACTIONS[a]
        if not action then
            error(string.format("Guardian statue action %d reads past its table", a))
        end
        action(me)
    end
    me:update_sprite()
    me.collision:present()
end

-- `sub_80D4CA4`: on its panel, facing its way, in the chip's flash; its
-- collision off the panels until it stands. (Shown from its first update.)
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:step()
    sprite:set_flip(me.flip)
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me.name_id = NAME
    me.hp, me.max_hp = 1, 1
    me.timer = LIFETIME
    me.timer2 = APPEAR_TICKS
    if me:param(1) ~= 1 then
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), ARRIVAL)
    end
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(SELF_TYPE, TARGET_TYPE, HIT_MOD)
    me.collision.region = 0
    me.collision:present()
    me.lifecycle = "update"
end

-- `sub_80D4C84`.
function guardian_statue.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return guardian_statue

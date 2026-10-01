--!strict
-- The Guardian statue (the original's attack object #0x7D, `sub_80D4C84`;
-- docs/engine/dimming-chip-effects.md §5): a field obstacle with 1 HP that
-- stands for 6000 ticks. Broken by an attack from one side only, it takes
-- the other side's part: in a dimming of its own (./strike, which the
-- other player can't cut in on) it strikes every panel the attacker's navi
-- stands on. Broken otherwise (by a body, by hits from both sides, by its
-- time running out) it just crumbles. The Guardian chip places one
-- (./controller, with the user's side); stages can place one at the start
-- (the kind's `place`, with its panel's side).
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions.

local absorbed_obstacle = require("../../objects/absorbed-obstacle/absorbed_obstacle")
local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("../../lib/regions")
local rising_bubble = require("../../objects/rising-bubble/rising_bubble")
local strike = require("./strike")

local statue = {}

export type State = {
    -- Placed by a stage (the original's first parameter 1): it takes its
    -- panel's side and runs while paused, and arrives without the flash.
    staged: boolean,
    -- (The original's second parameter: breaking it never strikes back.
    -- Nothing sets it.)
    quiet: boolean,
    -- Its strike's dimming says strike now (the original's third
    -- parameter).
    strike: boolean,
    -- What hit it when it broke (the collision's hit flags; the object's
    -- +0x60): which side strikes back.
    attackers: number,
}

local SPRITE = asset.sprite("guardian-statue")
-- Its NameID (the field objects' look) and its kind among absorbed
-- obstacles.
local NAME = 0xE3
-- How it looks absorbed (the original's obstacle kind's sprite: its own).
local ABSORBED = absorbed_obstacle.look { sprite = SPRITE }
local LIFETIME = 0x1770
-- It takes its panel on its third update.
local APPEAR_TICKS = 3
-- Where it can stand: a solid panel with nothing on it.
local SOLID, OCCUPIED = 0x10, 0x0F88_0080
-- The damage the stages' statues strike with.
local STAGE_DAMAGE = 200
local SOUND_PLACE = asset.sound("appear")
local SOUND_CRUMBLE = asset.sound("panel-poison")
local SOUND_STRIKE = asset.sound("beast-over-burst")
-- The flash it arrives in, 16 pixels up (effect 0x15).
local ARRIVAL = define.effect { sprite = asset.sprite("puff"), anim = 3, palette = 0 }
-- What it is and reaches: a neutral obstacle that everything reaches
-- (row 0x0E, while dimmed too, breaking), hit modifier 3.
local IS = define.collision { id = "guardian/statue", side0 = 0x00810082, side1 = 0x00810082, row_offset = 0x70 }
local HITS, HIT_MOD = collision.everything, 3
-- The collision types of each side's hits (`sub_80D4FF6`): side 0's
-- attacks, objects and bodies are the bits 0xAA000000, side 1's
-- 0x55000000.
local SIDE0_HITS, SIDE1_HITS = 0xAA00_0000, 0x5500_0000
-- The strike (`byte_80D4EB4`, by the side it strikes for): every panel with
-- the other side's player navi, a burst on each, the camera shaking, and
-- the statue in its striking animation. Its hit: an attack that pierces
-- invisibility and breaks (row 0x17), hitting navis, hit modifier 3, no
-- spark.
local STRIKE_REGION: { [number]: Region } = {
    [0] = define.region { field = { require = 0x0020_0000, forbid = 0 } },
    [1] = define.region { field = { require = 0x0040_0000, forbid = 0 } },
}
local STRIKE_IS = define.collision { id = "guardian/strike-hit", side0 = 0x8000008E, side1 = 0x4000008E, row_offset = 0xB8 }
local STRIKE_HITS, STRIKE_HIT_MOD = collision.hits_navis, 3
local STRIKE_BURST = define.effect { sprite = asset.sprite("beast-over-burst"), anim = 0, palette = 0 }
local STRIKE_ANIM = 1

-- Its actions (`off_80D4D60`).
local APPEAR, STANDING, STRIKE_BACK = 0, 8, 9

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

-- Gone: its collision off its panels, hidden, destroyed next update.
local function leave(me: Object)
    me.collision:set_region(nil)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- A puff 16 pixels up.
local function puff(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), effects.puff)
end

-- `sub_80D4F70`: crumble: the dust, a bubble, the sound.
local function crumble(me: Object)
    puff(me)
    rising_bubble.spawn(me.pos, 0)
    battle.play_sound(SOUND_CRUMBLE)
    leave(me)
end

-- `sub_80D4EFC`: leave the field; broken by one side's attack (its HP
-- run out), strike back for the other side instead.
local function destroyed(me: Object)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    local how = obstacle.removal(me)
    if how == "absorbed" then
        obstacle.fly_to_absorber(me, ABSORBED)
        leave(me)
        return
    elseif how ~= "broken" then
        local blink = obstacle.blink_out(me)
        if blink == "blinking" then
            return
        elseif blink == "done" then
            battle.play_sound(SOUND_CRUMBLE)
            leave(me)
            return
        end
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), effects.puff)
        crumble(me)
        return
    end
    local s = me.state :: State
    if me.hp == 0 and not s.quiet then
        local hits = me.collision.hit_flags
        s.attackers = hits
        if hits ~= 0 and avenging_side(hits) then
            me.action, me.phase, me.phase_init = STRIKE_BACK, 0, 0
            return
        end
    end
    crumble(me)
end

-- `sub_80D4D88`: on its third update, take the panel if it's free solid
-- ground; else crumble at once.
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
    me.collision:set_region(regions.single)
    battle.play_sound(SOUND_PLACE)
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
    me.collision:set_region(nil)
    local s = me.state :: State
    local side = if battle.over() then nil else avenging_side(s.attackers)
    if not side then
        puff(me)
        me.visible = false
        me:set_lifecycle("destroy")
        return
    end
    me.alliance = side
    -- (The HUD: sub_801DACC(0x40).)
    local controller = strike.spawn(me)
    dimming.start(side, true, controller, me)
    me.phase = 4
end

-- `sub_80D4E64`: once its dimming's effect says so, strike.
local function hit(me: Object)
    if not (me.state :: State).strike then
        return
    end
    local region = STRIKE_REGION[me.alliance]
    local h = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = region,
        hit_effect = nil,
        target = STRIKE_HITS,
        self_type = STRIKE_IS,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = STRIKE_HIT_MOD,
    })
    if h then
        h.run_while_dimmed = true
    end
    battle.region_effects(me.panel_x, me.panel_y, region, me.alliance, STRIKE_BURST, 0)
    battle.shake_camera(1, 0x3C)
    battle.play_sound(SOUND_STRIKE)
    me.anim = STRIKE_ANIM
    me.phase = 8
end

-- `sub_80D4EBC`: after the strike's animation, the dust.
local function after_strike(me: Object)
    if not bit32.btest(me.sprite.frame_flags, 0x80) then
        return
    end
    puff(me)
    me.phase = 0xC
end

-- `sub_80D4EE0`: gone, once the battle isn't dimmed.
local function strike_done(me: Object)
    me.collision:set_region(nil)
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
        hit(me)
    elseif phase == 8 then
        after_strike(me)
    elseif phase == 0xC then
        strike_done(me)
    else
        error(string.format("the Guardian statue's strike phase %#x reads past its table", phase))
    end
end

-- The statue's action table (`off_80D4D60`): its own actions and the
-- framework's shared ones (a push knocks it back as far as the hit says).
local ACTIONS: { [number]: (Object) -> () } = {
    [APPEAR] = appear,
    [1] = obstacle.return_to_idle,
    [2] = destroyed,
    [3] = obstacle.flinch,
    [4] = obstacle.paralyzed,
    [5] = obstacle.knocked_back,
    [6] = obstacle.frozen,
    [7] = obstacle.bubbled,
    [STANDING] = standing,
    [STRIKE_BACK] = strike_back,
}

-- `sub_80D4D34`: hits (a push keeps the damage), the lifetime, the
-- reaction (a crushing touch destroys it, its HP as it is), its action.
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
-- collision off the panels until it stands. (Shown from its first update,
-- which is the next tick's.)
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:step()
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me.name_id = NAME
    me.hp, me.max_hp = 1, 1
    me.timer = LIFETIME
    me.timer2 = APPEAR_TICKS
    if not (me.state :: State).staged then
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), ARRIVAL)
    end
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(IS, HITS, HIT_MOD)
    me.collision:set_region(nil)
    me.collision:present()
    me.lifecycle = "update"
end

-- The kind (below; its `place` spawns one).
local KIND: Kind

-- `sub_80D4FA6`: a statue on (x, y) with the damage word `damage`: placed
-- by `owner`, of its side and facing; placed by a stage (`owner` nil), of
-- the panel's side, running while paused too. Its side's one field object
-- of class 1, running while dimmed. (Until its init places it, its
-- position is `pos`, the spawner's registers.)
function statue.spawn(owner: Object?, x: number, y: number, damage: number, pos: Vec3): Object?
    local o = battle.spawn(KIND, pos)
    if not o then
        return nil
    end
    local s = o.state :: State
    s.staged = owner == nil
    o.panel_x, o.panel_y = x, y
    o:set_damage_word(damage)
    if owner then
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

KIND = define.kind {
    id = "guardian/statue",
    pool = "attack",
    state = { staged = "bool", quiet = "bool", strike = "bool", attackers = "u32" },
    -- `sub_80D4C84`.
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
    -- `sub_800751C`: a stage's statue (an actor list's entry type 9), on
    -- its panel's side, striking for 200. The entry's argument only reaches
    -- the position the spawner leaves before its init.
    place = function(spec: PlaceSpec): Object?
        local x, y = spec.panel_x, spec.panel_y
        local panel = field.panel(x, y)
        if not panel then
            error(string.format("a stage places a Guardian statue off the field at (%d, %d)", x, y))
        end
        return statue.spawn(nil, x, y, STAGE_DAMAGE, Vec3.new(y, panel.alliance, spec.argument))
    end,
}
statue.kind = KIND

return statue

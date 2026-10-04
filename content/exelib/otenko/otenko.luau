--!strict
-- Otenko's statue and dimming controller, as BN5 and BN6 share them (BN6's attack object #0xAD, `sub_80DB0E4`, and
-- effect object #0x5F, `sub_80E7624`; BN5's #0xAD, 0x080DBB84, and #0x5F, 0x080E7518; BN6's
-- docs/engine/dimming-chip-effects.md §9): the controller places a statue in front of the user (if the panel is
-- solid and free) and waits 60 ticks; the statue, a field obstacle with 100 HP, stands for its lifetime and, every 5
-- ticks, adds 1 to the Atk+ bonus of the chip at the user's hand's cursor if it deals damage, up to 50 for each hand
-- entry.
--
-- An obstacle: its update runs the obstacle framework's steps (the `obstacle` service) around its own actions.
-- Bodies don't break it, and nothing pushes it.
--
-- Each game makes its own (`otenko.make`) with its look: the sprite, identity, lifetime (BN6's 1800, BN5's 600),
-- the registry's class (BN6's 0, two a side; BN5's 1), the sounds, how it leaves (BN6's `sub_80DB252`: absorbed,
-- blinking out, or a puff or bubbles with the breaking sound; BN5's 0x080DBD1A: a puff with HP left, else bubbles,
-- silent, whatever removed it), and whether its blessing goes to the side's special bonus with each side keeping its
-- own gauge (BN5's 0x080DBC78, battle flag 0x40).

local dimming_chips = require("@exelib/dimming")
local regions = require("@exelib/regions")

local otenko = {}

export type State = {
    -- The hand entry it last blessed (ExtraVars+0), that entry's turn (+4), and how many times in a row (+8).
    entry: number,
    turn: number,
    count: number,
}

export type Look = {
    -- The kinds' ids.
    statue_id: string,
    controller_id: string,
    sprite: SpriteAsset,
    identity: Identity,
    lifetime: number,
    class: number,
    sounds: { appear: Sound, ["break"]: Sound? },
    -- What it is and reaches: another body of its side, hit by the other side.
    is: CollisionType,
    hits: CollisionType,
    -- How it looks absorbed (BN6's: absorbed, it flies to the absorber, and a removal may blink it out); none
    -- (BN5's): a removal leaves a puff.
    absorbed: AbsorbedLook?,
    -- The effects it leaves (the game's lib/effects).
    effects: { puff: Effect, bubbles: Effect },
    -- BN5's: with each side keeping its own gauge, the side's special bonus gets the blessing.
    special_bonus: boolean,
}

export type Otenko = {
    statue: Kind,
    -- The chip's `dimming` hook (`sub_80E76D4`).
    hook: (user: Object, spec: DimmingChipSpec) -> Object?,
}

local HP = 100
-- It takes its panel after 5 ticks; it blesses every 5.
local APPEAR_TICKS, EVERY = 5, 5
local BLESSING_ANIM = 1
-- Each hand entry gains at most this much.
local MOST = 50
-- Where it can stand: a solid panel without a body.
local SOLID, BODIES = 0x10, 0x0F80_0000
local HIT_MOD = 3
-- The controller's wait, and where it places the statue (`byte_80E76C4`, both sides'): a solid panel with nothing
-- on it.
local WAIT = 0x3C
local OCCUPIED = 0x0F88_0080

function otenko.make(look: Look): Otenko
    local absorbed = look.absorbed

    -- `sub_80DB198`: if its panel is solid without a body, it takes it after 5 ticks; else it breaks.
    local function appear(me: Object)
        if me.phase_init == 0 then
            if not field.check(me.panel_x, me.panel_y, SOLID, BODIES) then
                me:set_action(2)
                return
            end
            me.timer2 = APPEAR_TICKS
            battle.play_sound(look.sounds.appear)
            me.phase_init = 4
        end
        local t = me.timer2 - 1
        me.timer2 = t
        if t > 0 then
            return
        end
        me.collision:set_region(regions.single)
        -- (BN5's action table has no frozen or bubbled rows: its blessing is its action 6.)
        me:set_action(8)
    end

    -- `sub_80DB1E0`: every 5 ticks (from the first), +1 to the Atk+ bonus of the user's next chip if it deals damage,
    -- up to 50 an entry; (BN5's) with each side keeping its own gauge, +1 to the side's special bonus, up to 50.
    local function bless(me: Object)
        local special = look.special_bonus and battle.own_gauges()
        if me.phase_init == 0 then
            me.anim = BLESSING_ANIM
            me.timer2 = 0
            me.phase_init = 4
        end
        local t = me.timer2 - 1
        me.timer2 = t
        if t > 0 then
            return
        end
        me.timer2 = EVERY
        local s = me.state :: State
        local side = me.alliance
        if special then
            -- (BN5 also restarts the count when the side state's +0x53 is set, clearing it: unverified, nothing in
            -- the engine models that byte, and battle flag 0x40 is never set in a netbattle.)
            if s.count >= MOST then
                return
            end
            battle.add_special_bonus(side, 0, 1)
            s.count += 1
            return
        end
        local user = me.related1
        if not user then
            error("Otenko's statue has no user (sub_800ED90 reads through a null pointer)")
        end
        if not battle.next_chip_damages(user) then
            return
        end
        local i = battle.hand_cursor(side)
        local turn = battle.hand_turn(side, i)
        local same = i == s.entry and turn == s.turn
        s.entry, s.turn = i, turn
        if not same then
            s.count = 0
        elseif s.count >= MOST then
            return
        end
        battle.add_hand_attack_bonus(side, i, 1)
        s.count += 1
    end

    -- `sub_80DB252`: leave the field: (BN6's) absorbed, blinking out, or with a puff (with HP left) or bubbles
    -- (broken).
    local function destroyed(me: Object)
        me.collision:set_region(nil)
        me:unreserve_panel(me.future_panel_x, me.future_panel_y)
        obstacle.release_tracking(me)
        obstacle.unregister(me)
        if absorbed and obstacle.removal(me) == "absorbed" then
            obstacle.fly_to_absorber(me, absorbed)
        else
            local blink = if absorbed then obstacle.blink_out(me) else "no"
            if blink == "blinking" then
                return
            elseif blink == "no" then
                local p = me.pos
                if me.hp ~= 0 then
                    battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), look.effects.puff)
                else
                    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), look.effects.bubbles)
                    if look.sounds["break"] then
                        battle.play_sound(look.sounds["break"])
                    end
                end
            end
        end
        me.visible = false
        me:set_lifecycle("destroy")
    end

    -- The statue's action table (`off_80DB174`): its own actions and the framework's shared ones (a push slides it
    -- up to six panels).
    local actions: { [number]: (Object) -> () } = {
        [0] = appear,
        [1] = obstacle.return_to_idle,
        [2] = destroyed,
        [3] = obstacle.flinch,
        [4] = obstacle.paralyzed,
        [5] = obstacle.slide,
        [6] = obstacle.frozen,
        [7] = obstacle.bubbled,
        [8] = bless,
    }

    -- `sub_80DB148`: hits (never pushed), the lifetime, the reaction (bodies don't break it), its action; then the
    -- sprite, and present.
    local function tick(me: Object)
        me.visible = true
        me.sprite.white = false
        obstacle.take_hits(me, "ignored")
        obstacle.tick_lifetime(me)
        local a = obstacle.react(me, "spares_bodies")
        if a then
            local action = actions[a]
            if not action then
                error(string.format("Otenko statue action %d reads past its table", a))
            end
            action(me)
        end
        me:update_sprite()
        me.collision:present()
    end

    -- `sub_80DB108`: on its panel, its collision on no panel yet; on at once.
    local function init(me: Object)
        me:set_coordinates_from_panel()
        me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "ground"
        me.anim, me.anim_loaded = 0, 0xFF
        if not me:create_collision() then
            -- (The original skips the flip and presents through a null pointer.)
            error("Otenko's statue has no collision: sub_80DB108 presents through a null pointer")
        end
        me:setup_collision(look.is, look.hits, HIT_MOD)
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me.collision:present()
        me.timer = look.lifetime
        me.identity = look.identity
        me.hp, me.max_hp = HP, HP
        local s = me.state :: State
        s.entry, s.count, s.turn = 0xFF, 0, 0
        me:set_lifecycle("update")
        tick(me)
    end

    local statue = define.kind {
        id = look.statue_id,
        pool = "attack",
        state = { entry = "u32", turn = "u32", count = "u32" },
        -- `sub_80DB0E4`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
                return
            end
            me:update_sprite()
        end,
    }

    -- `sub_80DB2C6`: a statue on (x, y) with the damage word, of `user`'s side and facing, element 0, running while
    -- dimmed; one of its side's field objects of the look's class. (Its position is the spawner's registers until its
    -- init places it.)
    local function spawn(user: Object, x: number, y: number, damage: number): Object?
        local o = battle.spawn(statue)
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = x, y
        o.element = 0
        o:set_damage_word(damage)
        o.alliance, o.flip = user.alliance, user.flip
        o.related1 = user
        o.run_while_dimmed = true
        obstacle.register(o, user.alliance, look.class)
        return o
    end

    -- `sub_80E7668`: the statue, then 60 ticks.
    local function effect(me: Object)
        if me.phase_init == 0 then
            local x, y = me.panel_x + me.facing, me.panel_y
            if field.check(x, y, SOLID, OCCUPIED) then
                local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
                spawn(me.related1 :: Object, x, y, damage)
            end
            me.timer = WAIT
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t <= 0 then
            dimming_chips.done(me)
        end
    end

    local controller = define.kind {
        id = look.controller_id,
        pool = "effect",
        -- `sub_80E7624`.
        update = dimming_chips.phases { name = "Otenko", effect = effect },
    }

    -- The chip's `dimming` hook (`sub_80E76D4`).
    local function hook(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(controller, user, spec)
    end

    return { statue = statue, hook = hook }
end

return otenko

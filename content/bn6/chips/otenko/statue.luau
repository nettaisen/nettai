--!strict
-- Otenko's statue (the original's attack object #0xAD, `sub_80DB0E4`;
-- docs/engine/dimming-chip-effects.md §9): a field obstacle with 100 HP
-- that stands for 1800 ticks in front of the user and, every 5 ticks,
-- adds 1 to the Atk+ bonus of the chip at the user's hand's cursor if it
-- deals damage, up to 50 for each hand entry.
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions. Bodies don't break it, and
-- nothing pushes it.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("../../lib/regions")

local statue = {}

export type State = {
    -- The hand entry it last blessed (ExtraVars+0), that entry's turn
    -- (+4), and how many times in a row (+8).
    entry: number,
    turn: number,
    count: number,
}

-- (`sub_8011504`'s word: sprite 0C-00, with a shadow, animation 0.)
local SPRITE = asset.sprite("sword")
local NAME = 0xCF
local LIFETIME, HP = 0x708, 100
-- It takes its panel after 5 ticks; it blesses every 5.
local APPEAR_TICKS, EVERY = 5, 5
local BLESSING_ANIM = 1
-- Each hand entry gains at most this much.
local MOST = 50
-- Where it can stand: a solid panel without a body.
local SOLID, BODIES = 0x10, 0x0F80_0000
local SOUND_APPEAR = asset.sound("appear")
local SOUND_BREAK = asset.sound("panel-poison")
-- What it is and reaches: another body of its side, hit modifier 3.
local IS, HITS, HIT_MOD = collision.own_body, collision.hit_by_other_side, 3
-- Its kind among absorbed obstacles.
local ABSORBED_KIND = 0xE

-- `sub_80DB198`: if its panel is solid without a body, it takes it after 5
-- ticks; else it breaks.
local function appear(me: Object)
    if me.phase_init == 0 then
        if not field.check(me.panel_x, me.panel_y, SOLID, BODIES) then
            me:set_action(2)
            return
        end
        me.timer2 = APPEAR_TICKS
        battle.play_sound(SOUND_APPEAR)
        me.phase_init = 4
    end
    local t = me.timer2 - 1
    me.timer2 = t
    if t > 0 then
        return
    end
    me.collision:set_region(regions.single)
    me:set_action(8)
end

-- `sub_80DB1E0`: every 5 ticks (from the first), +1 to the Atk+ bonus of
-- the user's next chip if it deals damage, up to 50 an entry.
local function bless(me: Object)
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
    local user = me.related1
    if not user then
        error("Otenko's statue has no user (sub_800ED90 reads through a null pointer)")
    end
    if not battle.next_chip_damages(user) then
        return
    end
    local s = me.state :: State
    local side = me.alliance
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

-- `sub_80DB252`: leave the field: absorbed, blinking out, or with a puff
-- (removed with HP left) or bubbles (broken).
local function destroyed(me: Object)
    me.collision:set_region(nil)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    if obstacle.removal(me) == "absorbed" then
        obstacle.fly_to_absorber(me, ABSORBED_KIND)
    else
        local blink = obstacle.blink_out(me)
        if blink == "blinking" then
            return
        elseif blink == "no" then
            local p = me.pos
            if me.hp ~= 0 then
                battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), effects.puff)
            else
                battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), effects.bubbles)
                battle.play_sound(SOUND_BREAK)
            end
        end
    end
    me.visible = false
    me:set_lifecycle("destroy")
end

-- The statue's action table (`off_80DB174`): its own actions and the
-- framework's shared ones (a push slides it up to six panels).
local ACTIONS: { [number]: (Object) -> () } = {
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

-- `sub_80DB148`: hits (never pushed), the lifetime, the reaction (bodies
-- don't break it), its action; then the sprite, and present.
local function tick(me: Object)
    me.visible = true
    me.sprite.white = false
    obstacle.take_hits(me, "ignored")
    obstacle.tick_lifetime(me)
    local a = obstacle.react(me, "spares_bodies")
    if a then
        local action = ACTIONS[a]
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
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0xFF
    if not me:create_collision() then
        -- (The original skips the flip and presents through a null
        -- pointer.)
        error("Otenko's statue has no collision: sub_80DB108 presents through a null pointer")
    end
    me:setup_collision(IS, HITS, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.collision:present()
    me.timer = LIFETIME
    me.name_id = NAME
    me.hp, me.max_hp = HP, HP
    local s = me.state :: State
    s.entry, s.count, s.turn = 0xFF, 0, 0
    me:set_lifecycle("update")
    tick(me)
end

statue.kind = define.kind {
    id = "otenko/statue",
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

-- `sub_80DB2C6`: a statue on (x, y) with the damage word, of `user`'s side
-- and facing, element 0, running while dimmed; one of its side's two field
-- objects of class 0 (a third evicts the oldest). (Its position is the
-- spawner's registers until its init places it.)
function statue.spawn(user: Object, x: number, y: number, damage: number): Object?
    local o = battle.spawn(statue.kind)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = 0
    o:set_damage_word(damage)
    o.alliance, o.flip = user.alliance, user.flip
    o.related1 = user
    o.run_while_dimmed = true
    obstacle.register(o, user.alliance, 0)
    return o
end

return statue

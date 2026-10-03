--!strict
-- The computer navi's buster (./ai's buster run, 0x0802BB7A): the original's weapon routine 0x3E (0x0800FB06),
-- Attack plus one, and its action 0x16 (0x080ED098), BN5's own: a punch. The navi swings a fist (attachment row
-- 0x10, its animation by the actor's AI index) with a swing drawn where it stands (effect 0x27), and on the swing's
-- tenth tick a pushing hit lands on the column of panels ahead (hit region 0x1205FF04: region 4, no spark, target
-- row 5, self row 0x12; hit modifier 0x49), which stays on the field up to ten ticks until it hits; after the
-- swing's 22 ticks, six of rest.

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local regions = require("@common/regions")

local punch = {}

export type State = {
    -- Ticks left of the swing, then of the rest (AIAttackVars+0x10).
    timer: number,
}

-- The fist (the attachment table's row 0x10, 0x080B9974).
local FIST = attachment.look { sprite = asset.sprite("bn5:punch"), palette = 0 }
-- The swing (the effect table's row 0x27, 0x080DFC94).
local SWING = define.effect { sprite = asset.sprite("bn5:punch-swing"), anim = 0, palette = 0 }
local SOUND = asset.sound("bn5:windrack")

-- The phases, on the attack's step.
local PUNCH, REST = 0x0, 0x4

-- 0x080ED17A: the fist's animation, by the actor's AI index (a virus's 0; a navi's but 0x16's, Dark MegaMan's, 0;
-- a player's but 0x17's, 0).
local function fist_anim(me: Object): number
    local kind, ai = me.actor_type, me.ai_index
    if kind == "virus" then
        return 0
    elseif kind == "navi" then
        return if ai == 0x16 then 0 else ai
    end
    return if ai == 0x17 then 0 else ai
end

-- `object_getFlip`: the way the navi faces, as the game's flip value.
local function flip(me: Object): number
    return bit32.bxor(me.alliance, me.flip)
end

-- 0x080ED0B4: the swing; the hit on its tenth tick (the timer at 12).
local function swing(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(5)
        me:open_counter_window()
        battle.play_sound(SOUND)
        attachment.attach(me, FIST, "related", { anim = fist_anim(me) })
        me:set_status("using_action", true)
        local cx, cy = field.center(me.panel_x, me.panel_y)
        battle.effect(Vec3.new(cx, cy, 0x10 * 0x10000), SWING, flip(me))
        s.timer = 21
        me.step_init = 4
    end
    if s.timer == 12 then
        -- The damage word: the attack's damage plus its Atk+ bonus, the counter byte above.
        local box = battle.hitbox(me, {
            panel_x = me.panel_x + me.facing,
            panel_y = me.panel_y,
            element = 0,
            z = 0x10 * 0x10000,
            region = regions.wide,
            hit_effect = nil,
            target = collision.hits_navis,
            self_type = collision.pushing,
            damage = me.attack_damage + me.extra,
            stamina = me.hit_param,
            hit_mod = 0x49,
        })
        if box then
            box.timer = 10
        end
    end
    local left = s.timer - 1
    s.timer = left
    if left < 0 then
        me.step = REST
        me.step_init = 0
    end
end

-- 0x080ED154: six ticks of rest, then the fist goes and the attack ends.
local function rest(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = 5
        me.step_init = 4
    end
    local left = s.timer - 1
    s.timer = left
    if left < 0 then
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end
end

-- 0x080ED098.
local function update(me: Object, s: State)
    local step = me.step
    if step == PUNCH then
        swing(me, s)
    elseif step == REST then
        rest(me, s)
    else
        error(string.format("the punch's phase %#x reads past its phase table (0x080ED0AC)", step))
    end
end

punch.action = define.action { id = "bn5:computer-navi/punch", state = { timer = "u16" }, update = update }

-- 0x0800FB06: Attack plus one, no counter byte.
function punch.setup(navi: Object): Action
    navi.attack_damage = battle.navi(navi.alliance).attack + 1
    navi.hit_param = 0
    return punch.action
end

punch.weapon = define.weapon {
    id = "bn5:computer-navi/punch",
    -- The charge table's row 0x3E (0x0801CCD8); Charge 5 reads on into row 0x3F's first entry.
    charge_ticks = { 60, 60, 60, 60, 60, 110 },
    setup = punch.setup,
}

return punch

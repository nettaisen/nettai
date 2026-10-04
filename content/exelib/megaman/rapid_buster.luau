--!strict
-- The rapid buster, as BN5 and BN6 share it (BN6's action 0x5D, `sub_80F00F2`, and its weapon routines 0x39 and
-- 0x92, `sub_8012124`; BN5's action 0x2E, 0x080ED9B8, and its weapon routine 0x68, 0x0800FB32, the patch cards'
-- Sbustr: the same code but for the muzzle flash's attachment row, 13 for BN6's 5, and how the form overlay restarts),
-- of a game's `RapidBusterLook` (`rapid_buster.make`): nine charged shots, six ticks apart, each from the panel in
-- front and a row up, down or level in an order one draw picks. See docs/engine/shot-chips.md §11.

local attachment = require("../attachment")

local rapid_buster = {}

export type State = {
    -- The shot's count (AIAttackVars+0x10), and the rows' order
    -- (AIAttackVars+0x30, a pointer into `byte_80F0138`: its row).
    timer: number,
    pattern: number,
}

-- The phases, on the attack's step.
local PATTERN, FIRE, RECOVER = 0x0, 0x4, 0x8

local ANIM_SHOOT = 0x0E
local SHOTS, SHOT_TICK, SHOT_TICKS, RECOVER_TICKS = 9, 1, 5, 5
local SHOT_Z = 0x18_0000
-- The rows' orders (`byte_80F0138`): the six orders of -1, 0, +1.
local PATTERNS: { [number]: { number } } = {
    [0] = { -1, 1, 0 },
    [1] = { -1, 0, 1 },
    [2] = { 0, 1, -1 },
    [3] = { 0, -1, 1 },
    [4] = { 1, 0, -1 },
    [5] = { 1, -1, 0 },
}
-- Its damage: 5 x (the buster's Attack + 1), hit parameter 0x85.
local DAMAGE_PER_ATTACK, HIT_PARAM = 5, 0x85

-- A game's rapid buster (its action keyed `look.id`): its setup (`sub_8012124`: the attack's damage, no shots yet;
-- the action) and the action.
function rapid_buster.make(look: RapidBusterLook): { setup: (navi: Object) -> Action, action: Action }
    local made = {}

    -- `sub_80F014C`: each shot's entry raises the buster (the first) or
    -- restarts the arm and the form overlay; the shot on the count's 1, with
    -- the muzzle flash; after six ticks the next shot, or the recovery.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM_SHOOT)
            me.anim_loaded = 0xFF
            if me.attack_count == 0 then
                look.buster.raise_arm(me)
            else
                -- `sub_80B8E7C`, `sub_80C44D2` (BN6's steps the overlay at once; BN5's has it reload its
                -- animation at its next step).
                local arm = me.overlay
                if arm then
                    arm.anim_loaded = 0xFF
                end
                local overlay = me.related2
                if overlay then
                    overlay.anim_loaded = 0xFF
                    if look.overlay_steps then
                        overlay:step_sprite()
                    end
                end
            end
            me:set_status("using_action", true)
            s.timer = 0
            me.step_init = 4
        end
        if s.timer == SHOT_TICK then
            battle.play_sound(look.shot_sound)
            local row = PATTERNS[s.pattern][me.attack_count % 3 + 1]
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000)
            look.projectiles.spawn(me, me.panel_x + me.facing, me.panel_y + row, { variant = look.shot, damage = damage, z = SHOT_Z })
            attachment.attach(look.attachment, me, look.buster.flash, "related")
        end
        local t = s.timer + 1
        s.timer = t
        if t <= SHOT_TICKS then
            return
        end
        local shots = me.attack_count + 1
        me.attack_count = shots
        if shots < SHOTS then
            me.step_init = 0
        else
            me.step, me.step_init = RECOVER, 0
        end
    end

    -- `sub_80F0110`: one draw picks the rows' order; the shots start at once.
    local function pattern(me: Object, s: State)
        s.timer = 0
        s.pattern = battle.rng_positive() % 6
        me.step, me.step_init = FIRE, 0
        fire(me, s)
    end

    -- `sub_80F01E8`: 6 ticks, then the buster goes and back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me.related1 = nil
            me.overlay = nil
            me:exit_attack()
        end
    end

    made.action = define.action {
        id = look.id,
        state = { timer = "u16", pattern = "u8" },
        -- `sub_80F00F2`.
        update = function(me: Object, s: State)
            local step = me.step
            if step == PATTERN then
                pattern(me, s)
            elseif step == FIRE then
                fire(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("rapid buster phase %#x reads past off_80F0104", step))
            end
        end,
    }

    -- `sub_8012124`: the attack's damage, no shots yet; the action.
    function made.setup(navi: Object): Action
        navi.attack_damage = (battle.navi(navi.alliance).attack + 1) * DAMAGE_PER_ATTACK
        navi.hit_param = HIT_PARAM
        navi.attack_count = 0
        return made.action
    end

    return made
end

return rapid_buster

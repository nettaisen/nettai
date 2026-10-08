--!strict
-- CopyDmg (EXE4 chip 0x094; its record at 0x080197EC + 0x94 * 0x2C): action 0x2B (0x080ED1D2: EXE4's own; EXE5's
-- action 0x24 grew from it). The navi takes animation 0x12 (no arm raised) with a counter window and, on the next
-- tick, with the buster's sound fires its mark (@exelib/copydmg/mark in EXE4's look, attack 0x28: 0x080D1C24) along
-- its row from the panel ahead, with the chip's damage word (no Atk+ bonus); 5 ticks, then 11, and back to idle. The
-- first navi the mark hits is marked for 90 ticks: it takes, on top of its own, the damage its side took the tick
-- before (the side's damage-carry record, 0x0203C050: 0x08022258 in the intake); the mark shows at its position.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local common_mark = require("@exelib/copydmg/mark")

-- The mark (EXE4's attack 0x28, 0x080D1C24).
local mark = common_mark.make {
    id = "copydmg/mark",
    sprite = asset.sprite("hit-sparks"),
    anim = 0x0B,
    sound = asset.sound("mark"),
    is = collision.attack,
    hits = collision.hits_navis,
    ticks = 90,
    over_navi = "position",
    reload_anim = true,
    own_gauges_hit = nil,
}

local ANIM = 0x12
local SOUND = asset.sound("buster-shot")
local SHOOT, RECOVER = 0x0, 0x4
local RECOVER_TICKS = 0x0A

type State = {
    -- Ticks into the shot, then ticks left (AIAttackVars +0x10).
    timer: number,
}

-- 0x080ED1F0: the pose; the mark flies on the second tick.
local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(ANIM)
        me:open_counter_window()
        me.step_init = 4
        s.timer = 0
    end
    if s.timer == 1 then
        battle.play_sound(SOUND)
        -- The damage word (AIAttackVars +8: the hit parameter in its high half). The spawner's register r2 (its
        -- position's Y until its init places it, the same tick) is the action dispatcher's: 0 here.
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000)
        mark.spawn(me, me.panel_x + me.facing, me.panel_y, 0, damage)
    end
    local t = s.timer + 1
    s.timer = t
    if t > 4 then
        me.step, me.step_init = RECOVER, 0
    end
end

-- 0x080ED23A: 11 ticks, then back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t < 0 then
        me.related1 = nil
        me:exit_attack()
    end
end

local copydmg: Chip = {
    action = new.action {
        id = "copydmg/action",
        state = { timer = "u16" },
        -- 0x080ED1D2.
        update = function(me: Object, s: State)
            local step = me.step
            if step == SHOOT then
                shoot(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("CopyDmg's phase %#x reads past its table (0x080ED1E4)", step))
            end
        end,
    },
    codes = { "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 12,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
}

return { copydmg = copydmg }

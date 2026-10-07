--!strict
-- MegaMan's charged shot in EXE4 (`megaman/charged-shot`): the original's weapon routine 1 (0x0800CC54: (Attack + 1) *
-- 10 damage, every attack variable 0) and the shot it starts (action 0x24, 0x080ECC2C), EXE4's own: as the buster's
-- shot (./buster) but always firing, the charged projectile (row 6) with the attack's damage alone (its halfword),
-- and after the recovery runs out a move may still start on the same tick.
--
-- Nearest the engine can say (docs/design/exe4-map.md §18): the attack's exits (0x0800CA28; 0x0800C9FC, which also
-- writes AIData +0x3A) and the move that cuts the recovery short as the engine's.

local attachment = require("../../../objects/attachment")
local projectile = require("../../../lib/projectile")
local variants = require("../../../objects/projectile/variants")

local charged_shot = {}

local SHOOT, RECOVER = 0x0, 0x4
local SHOT_Z = 0x18_0000
-- The flash's animation: NaviStats +0x23, the navi's number (MegaMan's 0).
local NAVI_NUMBER = 0
-- Its sound: the buster's shot (0x6A).
local SHOT_SOUND = asset.sound("buster-shot")

type State = { timer: number }

-- 0x080ECC4C.
local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x12)
        me.step_init = 4
        me:set_status("using_action", true)
        s.timer = 0
    end
    if s.timer == 1 then
        battle.play_sound(SHOT_SOUND)
        me.attack_count = projectile.fire(me, { variant = variants.charged, damage = me.attack_damage, z = SHOT_Z })
        attachment.attach(me, attachment.muzzle_flash, "related", NAVI_NUMBER)
    end
    s.timer += 1
    if s.timer > 4 then
        me.step = RECOVER
        me.step_init = 0
    end
end

-- 0x080ECCAC: the recovery; its end exits the attack and a move may still start, as may one before.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = me.attack_count
        me.step_init = 4
    end
    s.timer -= 1
    if s.timer < 0 then
        me.related1 = nil
        -- 0x080ECCC8: the end that keeps the lockouts (0x0800CA28).
        me:exit_attack(true)
    end
    if not me:can_move() then
        return
    end
    local dir = me:held_direction()
    if dir == 0 or me:step_target(dir) == nil then
        return
    end
    me.related1 = nil
    -- 0x080ECCE0: the move's hands the lockout on (0x0800C9FC).
    me:exit_attack()
    me:start_move(dir)
end

charged_shot.action = new.action {
    id = "megaman/charged-shot/action",
    state = { timer = "i16" },
    update = function(me: Object, s: State)
        local step = me.step
        if step == SHOOT then
            shoot(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("charged shot phase %#x reads past its phase table", step))
        end
    end,
}

-- 0x0800CC54.
function charged_shot.setup(navi: Object): Action
    navi.attack_damage = (battle.navi(navi.alliance).attack + 1) * 10
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    return charged_shot.action
end

charged_shot.weapon = new.weapon {
    id = "megaman/charged-shot",
    -- The charge table's row 1 (0x080180B8 by NaviStats +0x0A, 0x0800BD6A and 0x0800BD1C), by Charge 0 to 4; Charge
    -- 5 reads on into row 2's first entry.
    charge_ticks = { 120, 100, 85, 70, 60, 180 },
    setup = charged_shot.setup,
}

return charged_shot

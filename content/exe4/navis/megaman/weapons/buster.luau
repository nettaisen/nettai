--!strict
-- MegaMan's buster in EXE4 (`megaman/buster`): the original's weapon routine 0 (0x0800CC2E: Attack + 1 damage, every
-- attack variable 0) and the shot it starts (action 8, 0x080EB316), EXE4's own: no arm is raised (animation 0x12), and
-- on the shot's second tick a draw of the battle's RNG2 decides whether it fires: a draw of 1 to 16 no greater than
-- the navi's blank count (NaviStats +0x08, the NaviCust's and patch cards' bug) clicks (sound 0x1C0) and fires nothing;
-- else the shot (sound 0x6A), the plain projectile from the panel in front, and the muzzle flash in the navi's first
-- related slot (attachment row 0x10, its animation the navi's number). After five ticks, the recovery the shot
-- counted (a blank waits the last shot's), which a move may cut short.
--
-- Nearest the engine can say (docs/design/exe4-map.md §18): the attack's exit (0x0800CA28) and the move that cuts the
-- recovery short (0x0800AD2A, 0x0800B4B0, 0x0800C9BA) as the engine's.

local attachment = require("../../../objects/attachment")
local projectile = require("../../../lib/projectile")
local variants = require("../../../objects/projectile/variants")

local buster = {}

-- The phases (the attack's step).
local SHOOT, RECOVER = 0x0, 0x4
-- The shot's height, 16.16.
local SHOT_Z = 0x18_0000
-- The flash's animation: NaviStats +0x23, the navi's number (MegaMan's 0, the one navi EXE4's netbattles have).
local NAVI_NUMBER = 0
-- Its sounds: the shot (0x6A) and a blank's click (0x1C0).
local SHOT_SOUND = asset.sound("buster-shot")
local BLANK_SOUND = asset.sound("buster-blank")

-- (The timer is a signed halfword: the recovery counts it down past 0, `bge`.)
type State = { timer: number }

-- 0x080EB334.
local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x12)
        me:set_status("using_action", true)
        me.step_init = 4
        s.timer = 0
    end
    if s.timer == 1 then
        local blanks = battle.navi(me.alliance).buster_blanks
        if bit32.band(battle.rng(), 0xF) + 1 > blanks then
            battle.play_sound(SHOT_SOUND)
            local damage = bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16))
            me.attack_count = projectile.fire(me, { variant = variants.plain, damage = damage, z = SHOT_Z })
            attachment.attach(me, attachment.muzzle_flash, "related", NAVI_NUMBER)
        else
            battle.play_sound(BLANK_SOUND)
        end
    end
    s.timer += 1
    if s.timer > 4 then
        me.step = RECOVER
        me.step_init = 0
    end
end

-- 0x080EB3B6: the recovery; a move may cut it short.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = me.attack_count
        me.step_init = 4
    end
    s.timer -= 1
    if s.timer < 0 then
        me.related1 = nil
        -- 0x080EB3D2: the end that keeps the lockouts (0x0800CA28).
        me:exit_attack(true)
        return
    end
    if not me:can_move() then
        return
    end
    local dir = me:held_direction()
    if dir == 0 or me:step_target(dir) == nil then
        return
    end
    me.related1 = nil
    -- 0x080EB3EC: so does the move's.
    me:exit_attack(true)
    me:start_move(dir)
end

buster.shot = new.action {
    id = "megaman/buster/shot",
    state = { timer = "i16" },
    update = function(me: Object, s: State)
        local step = me.step
        if step == SHOOT then
            shoot(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("buster shot phase %#x reads past its phase table", step))
        end
    end,
}

-- 0x0800CC2E.
function buster.setup(navi: Object): Action
    navi.attack_damage = battle.navi(navi.alliance).attack + 1
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    return buster.shot
end

buster.weapon = new.weapon {
    id = "megaman/buster",
    -- The buster isn't charged (the charge table's row 0).
    charge_ticks = { 0, 0, 0, 0, 0 },
    setup = buster.setup,
}

return buster

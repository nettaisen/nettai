--!strict
-- GyroMan's buster (`gyroman/buster`): the original's weapon routine 0x43 (0x0800FB92). On a solid panel it is
-- MegaMan's buster (routine 0, navis/megaman/weapons/buster). Over a panel that isn't solid, in the air, it is his own
-- action 10 (0x080F0CAC): no element, his own counter byte, his Attack plus one for damage; he fires down
-- (animation 21, a sound) and a strike (./strike) lands on every panel that holds an enemy navi's body, wherever it
-- is; 10 ticks, then he stands (his tick has him flying again) with his animation left as set.

local megaman_buster = require("../megaman/weapons/buster")
local panels = require("@exelib/panels")
local strike = require("./strike")

local buster = {}

export type State = {
    -- AIAttackVars +0x10: the ticks left.
    timer: number,
}

local SOLID = 0x10
local HIT_PARAM = 0x85
local ANIM = 21
local TICKS = 10
local SOUND = asset.sound("col-army-2")
-- The panels holding the other side's navi's body, by side (0x080F0D3C).
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }
-- The strikes' first parameter (their hit modifier 0).
local STRIKE_VARIANT = 1

-- 0x080F0CF6: a strike on each panel holding an enemy navi, the list's last first.
local function strafe(me: Object)
    local list = panels.filtered_except(me, ENEMY_NAVI[me.alliance], 0)
    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000)
    for i = #list, 1, -1 do
        strike.spawn(me, panels.x(list[i]), panels.y(list[i]), me.attack_element, STRIKE_VARIANT, damage)
    end
end

buster.action = define.action {
    id = "gyroman/buster/action",
    state = { timer = "u16" },
    -- 0x080F0CAC
    update = function(me: Object, s: State)
        if me.step ~= 0 then
            error(string.format("GyroMan's shot from the air's step %#x reads past its table (0x080F0CC0)", me.step))
        end
        -- 0x080F0CC4
        if me.step_init == 0 then
            me.step_init = 4
            me.anim = ANIM
            battle.play_sound(SOUND)
            strafe(me)
            s.timer = TICKS
            return
        end
        local t = s.timer - 1
        s.timer = bit32.band(t, 0xFFFF)
        if t <= 0 then
            me.anim = 0
            me:end_attack()
        end
    end,
}

-- 0x0800FB92
local function setup(navi: Object): Action
    if bit32.band(field.flags(navi.panel_x, navi.panel_y), SOLID) ~= 0 then
        return megaman_buster.setup(navi)
    end
    navi.attack_element = 0
    navi.hit_param = HIT_PARAM
    navi.attack_damage = battle.navi(navi.alliance).attack + 1
    return buster.action
end

buster.weapon = define.weapon {
    id = "gyroman/buster",
    -- The charge table's row 0x43 (0x0801CD0A), which a buster doesn't read.
    charge_ticks = { 60, 60, 60, 60, 60, 80 },
    setup = setup,
}

return buster

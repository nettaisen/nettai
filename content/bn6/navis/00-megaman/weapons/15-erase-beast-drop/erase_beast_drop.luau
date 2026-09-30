--!strict
-- EraseCross Beast's charged Null chip: weapon routine 0x15 (`sub_8011D20`)
-- and action 0x46 (`sub_80EE840`). The navi raises its arms; when the
-- animation ends, a drop (objects/erase-drop) falls on the nearest enemy
-- ahead (the first column with one; its row, or the nearest row with
-- one), or three panels ahead if there is none; 21 ticks later it idles.

local weapon = require("../../../../lib/weapon")
local erase_drop = require("../../../../objects/erase-drop/erase_drop")

export type State = {
    -- AIAttackVars+0x10: ticks of recovery.
    timer: number,
}

local erase_beast_drop = {
    state = { timer = "u16" },
}

local ACTION = 0x46

-- The phases, on the attack's step.
local RAISE, DROP, RECOVER = 0x0, 0x4, 0x8

-- Cursor (secondary element 0x40), the hit's counter byte.
local CURSOR, HIT_PARAM = 0x40, 0x9E

-- A frame's "last" flag.
local LAST_FRAME = 0x80

-- What marks a panel with an enemy on it, by the navi's side, alone and
-- with the panel bit the column search wants too (`byte_80ED080`).
local ENEMY = { [0] = 0x0400_0000, [1] = 0x0800_0000 }
local ENEMY_COLUMN = { [0] = 0x0401_0000, [1] = 0x0801_0000 }

-- The rows tried in the enemy's column, as steps from the navi's row
-- (`byte_80EE904`): its own, above, below, two above, two below.
local ROW_STEPS = { 0, -1, 2, -3, 4 }

-- The drop's fall, in ticks.
local FALL_TICKS = 3

-- `sub_8011D20`: 70 damage and 30 more per point of buster Attack (up to
-- 5), Cursor.
function erase_beast_drop.setup(navi: Object): number
    navi.attack_damage = weapon.charge_damage(navi, 0x46, 0x1E)
    navi.hit_param = HIT_PARAM
    navi.attack_element = CURSOR
    return ACTION
end

-- `sub_80ED040`: the first column ahead with an enemy on it; else the
-- far column, and false.
local function enemy_column(me: Object): (number, boolean)
    local front = me.facing
    local x = me.panel_x
    local want = ENEMY_COLUMN[me.alliance]
    while true do
        x += front
        if x == 0 or x >= 7 then
            break
        end
        -- object_getPanelsInColumnFiltered
        for y = 1, 3 do
            if field.check(x, y, want, 0) then
                return x, true
            end
        end
    end
    return if front > 0 then 6 else 1, false
end

-- `sub_80EE860`: raise the arms; when the animation ends, the drop's.
local function raise(me: Object)
    if me.step_init == 0 then
        me:set_animation(0x12)
        me:open_counter_window()
        me:set_status("using_action", true)
        me:set_status("moving", true)
        me.step_init = 4
        return
    end
    if bit32.btest(me.sprite.frame_flags, LAST_FRAME) then
        me:set_animation(0x13)
        me.step = DROP
        me.step_init = 0
    end
end

-- `sub_80EE89C`: when the animation ends, the drop on the target.
local function drop(me: Object, s: State)
    if not bit32.btest(me.sprite.frame_flags, LAST_FRAME) then
        return
    end
    local x, found = enemy_column(me)
    local tx, ty
    if found then
        local y = me.panel_y
        local enemy = ENEMY[me.alliance]
        for _, step in ROW_STEPS do
            y += step
            if bit32.btest(field.flags(x, y), enemy) then
                tx, ty = x, y
                break
            end
        end
    end
    if tx == nil then
        tx, ty = me.panel_x + me.facing * 3, me.panel_y
    end
    local damage = bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16))
    erase_drop.spawn(me, { FALL_TICKS, 0, int.u8(tx :: number), int.u8(ty :: number) }, damage)
    battle.play_sound(0xC4)
    s.timer = 0x14
    me.step = RECOVER
    me.step_init = 0
end

-- `sub_80ED998`: 21 ticks, then idle.
local function recover(me: Object, s: State)
    local left = s.timer - 1
    s.timer = left
    if left >= 0 then
        return
    end
    me:set_status("moving", false)
    me:exit_attack()
end

-- `sub_80EE840`.
function erase_beast_drop.update(me: Object, s: State)
    local step = me.step
    if step == RAISE then
        raise(me)
    elseif step == DROP then
        drop(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("EraseCross Beast's drop phase %#x reads past its phase table (sub_80EE840)", step))
    end
end

return erase_beast_drop

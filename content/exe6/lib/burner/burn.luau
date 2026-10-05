--!strict
-- The burners' burn (the original's action 0x27, `sub_80ECD28`): FireBrn1
-- to 3, WideBrn1 to 3 and HeatCross's charged shot. The navi holds out a
-- burner (an attachment) and lights a flame (flame.luau) on the panel
-- ahead; it burns its ticks and spreads forward, cracking the panels it
-- leaves. The navi keeps the pose as long as the flame burns, the burner
-- roaring every 16 ticks, then recovers for 6. Each user composes its own
-- (`burn.action { burner = ..., flame = ... }`).

local attachment = require("../../objects/attachment")
local flame = require("./flame")

local burn = {}

export type State = {
    -- Ticks of the pose so far, then the recovery's (AIAttackVars+0x10).
    timer: number,
}

-- (The burner roars every 16 ticks of the navi's `attack_count`, which
-- the burn counts on from whatever the last action left there: the first
-- roar's tick depends on it.)

-- One state for every burn.
burn.STATE = { timer = "u16" }

-- The burners (the original's attachment rows 0x16 and 0x1E): FireBrn's
-- and WideBrn's, and HeatCross's, in their animation 1.
burn.burners = {
    fire = attachment.look { sprite = asset.sprite("burner"), palette = 0, attach_point = 0x14 },
    heat = attachment.look { sprite = asset.sprite("heatcross-burner"), palette = 0, attach_point = 0x16 },
}
local BURNER_ANIM = 1

-- The phases, on the attack's step.
local BURN, RECOVER = 0x0, 0x4

local SOUND_ROAR = asset.sound("roar")

-- A burn (`BurnSpec`, types.d.luau): the burner held and the flame lit;
-- `id`, the action's key when no chip definition holds it.
function burn.action(spec: BurnSpec): Action
    -- `sub_80ECD44`: light the flame, then hold the pose as long as it
    -- burns.
    local function burning(me: Object, s: State)
        if me.step_init == 0 then
            -- The Atk+ bonus goes into the attack's damage word.
            local word = me.attack_damage + me.hit_param * 0x10000 + me.extra
            me.attack_damage = bit32.band(word, 0xFFFF)
            me.hit_param = bit32.rshift(word, 16)
            flame.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, word, spec.flame)
            me.anim = 0x0A
            me:open_counter_window()
            attachment.attach(me, spec.burner, "overlay", { anim = BURNER_ANIM })
            me:set_status("using_action", true)
            s.timer = 0
            me.step_init = 4
        end
        local roar = bit32.band(me.attack_count + 1, 0xF)
        me.attack_count = roar
        if roar == 1 then
            battle.play_sound(SOUND_ROAR)
        end
        local t = s.timer + 1
        s.timer = t
        if t >= spec.flame.ticks then
            me.step = RECOVER
            me.step_init = 0
        end
    end

    -- `sub_80ECDD6`: 6 ticks, then the burner goes and back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = 5
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

    -- `sub_80ECD28`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == BURN then
            burning(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("FireBrn phase %#x reads past its phase table", step))
        end
    end

    return define.action { id = spec.id, state = burn.STATE, args = spec, update = update }
end

return burn

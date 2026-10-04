--!strict
-- CornFsta's farmer (the original's actor object #0x1E, `sub_80BCB50`;
-- docs/engine/dimming-chip-effects.md §16): a stand-in for the user
-- (@exelib/dimming/stand_in) who raises CornSht's corn gun and, on the tenth
-- tick, sends a sower (./sower) out from the panel in front; once the
-- sower is done, 31 ticks, then the user comes back. (He is always
-- shown: no blind-viewer step.)

local stand_in = require("@exelib/dimming/stand_in")
local attachment = require("../../objects/attachment/attachment")
local cornsht = require("../cornsht/action")
local sower = require("./sower")

local farmer = {}

export type State = {
    -- The chip's controller (@exelib/dimming/stand_in).
    controller: Object?,
    -- The corn gun (ExtraVars+4).
    held: Object?,
    -- The sower is out (the original's fourth parameter, whose address
    -- it holds).
    sowing: boolean,
}

local ANIM_HOLD = 0xA
local FIRE_TICKS, AFTER = 10, 0x1E
local SOUND = asset.sound("bn6:corn-shot")

-- `sub_80BCC2C`: the gun; on the tenth tick the sower, the gun's next
-- animation and the sound.
local function fire(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_HOLD
        attachment.attach(me, cornsht.gun, "held", { anim = 0, while_dimmed = true })
        me.timer = 0
        me.phase_init = 4
    end
    local t = me.timer + 1
    me.timer = t
    if t < FIRE_TICKS then
        return
    end
    sower.spawn(me, me.panel_x + me.facing, me.panel_y, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)), me)
    local gun = (me.state :: State).held
    if gun then
        gun.anim += 1
    end
    battle.play_sound(SOUND)
    me.phase, me.phase_init = 4, 0
end

-- `sub_80BCC88`: until the sower is gone.
local function wait(me: Object)
    if not (me.state :: State).sowing then
        me.phase, me.phase_init = 8, 0
    end
end

-- `sub_80BCC96`: 31 ticks; then the gun goes and the user comes back.
local function done(me: Object)
    if me.phase_init == 0 then
        me.timer = AFTER
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t >= 0 then
        return
    end
    -- sub_80B8E58: the gun ends (its lifecycle word goes to destroy).
    local gun = (me.state :: State).held
    if gun then
        gun:set_lifecycle("destroy")
    end
    stand_in.leave(me)
end

-- `sub_80BCC0C`: by phase.
local function tick(me: Object)
    local phase = me.phase
    if phase == 0 then
        fire(me)
    elseif phase == 4 then
        wait(me)
    elseif phase == 8 then
        done(me)
    else
        error(string.format("CornFsta's farmer: phase %#x reads past off_80BCC20", phase))
    end
end

farmer.kind = define.kind {
    id = "bn6:cornfsta/farmer",
    pool = "actor",
    state = { controller = "object", held = "object", sowing = "bool" },
    -- `sub_80BCB50`: by state, then the sprite.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- sub_80BCB74: drawn as the user, who vanishes; on at once.
            stand_in.init(me)
            me:set_lifecycle("update")
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- `sub_80BCCDC`: the farmer for `user` on `controller`'s panel, with the
-- damage word `damage`. He takes the user's identity (the gun sits at that
-- sprite's attach point).
function farmer.spawn(controller: Object, user: Object, damage: number): Object?
    local o = stand_in.spawn(farmer.kind, controller, user, damage)
    if o then
        o.identity = user.identity
    end
    return o
end

return farmer

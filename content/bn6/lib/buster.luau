--!strict
-- What the buster's shots share (the buster, the charged shot, the blank
-- shot, DustCross's charged shot and the like): the arm the navi raises,
-- the recovery after a shot, and the move that may cut the recovery short.
-- See docs/engine/objects-and-player.md §B6.

local attachment = require("../objects/attachment/attachment")

local buster = {}

-- The muzzle flash a shot leaves in the navi's first related slot (the
-- original's attachment row 5, at attach point 0x10), and the buster arm a
-- thrown obstacle leaves there instead (row 6, the arm `raise_arm`
-- raises).
buster.flash = attachment.look { sprite = asset.sprite("muzzle-flash"), palette = 0, attach_point = 0x10 }
buster.arm = attachment.look { sprite = asset.sprite("buster-arm"), palette = 0 }
-- An enemy navi's arm (the original's attachment row 0x2B, at attach point
-- 0x0E).
local NAVI_ARM = attachment.look { sprite = asset.sprite("elec-coil"), palette = 0, attach_point = 0x0E }

-- The arm's palette additions: the charge disabled, and a Full Synchro
-- Beast form.
local NO_CHARGE_PALETTE = 0xE

-- How the arm animates, from the spawn parameters `sub_80EB572` adds into
-- one word: the attachment's row (the low byte, which nothing above it
-- changes), the animation shifted by 8, its r1 (Param3: the arm animates
-- while dimmed too) by 16 and the palette addition by 24 (so a negative
-- animation borrows from the bytes above it).
local function arm_options(
    anim: number,
    palette_add: number,
    param3: number
): { anim: number?, while_dimmed: boolean?, palette_add: number? }
    local word = int.u32(int.shl(anim, 8) + int.shl(param3, 16) + int.shl(palette_add, 24))
    return {
        anim = bit32.band(bit32.rshift(word, 8), 0xFF),
        while_dimmed = bit32.band(bit32.rshift(word, 16), 0xFF) ~= 0,
        palette_add = bit32.rshift(word, 24),
    }
end

-- `sub_80EB562`: raise the buster arm, kept in the navi's overlay slot. Its
-- animation shows the form; its palette the element, the charge disabled,
-- Full Synchro or the Beast form. A player's second actor gets none.
function buster.raise_arm(navi: Object)
    buster.raise_arm_for(navi, navi, "overlay", false)
end

-- `sub_80EB572`: the buster arm `navi`'s actor and side call for, held by
-- `holder` in its slot `which`, animating while dimmed too when
-- `while_dimmed`. A player's second actor gets none, and its overlay slot
-- is cleared. (CrosOver's MegaMan raises his user's arm this way.)
function buster.raise_arm_for(navi: Object, holder: Object, which: Slot, while_dimmed: boolean)
    local param3 = if while_dimmed then 1 else 0
    local function attach(look: AttachmentLook, anim: number, palette_add: number)
        attachment.attach(holder, look, which, arm_options(anim, palette_add, param3))
    end
    local kind = navi.actor_type
    if kind == "virus" then
        attach(buster.arm, 0, 0)
        return
    elseif kind == "navi" then
        attach(NAVI_ARM, navi.ai_index - 1, 0xD)
        return
    elseif navi.ai_index ~= 0 then
        navi.overlay = nil
        return
    end
    local side = navi.alliance
    local stats = battle.navi(side)
    local form = stats.form
    -- The form's arm (`buster_arm`): its animation, and what it adds to
    -- the palette (the Gregar Crosses in Beast Out: palettes 5 to 9).
    local arm = form.buster_arm
    local palette_add = 0
    if form.kind == "base" then
        if navi:navi_state("no_charge") then
            palette_add = NO_CHARGE_PALETTE
        elseif stats.element ~= 0 then
            palette_add = 0x14 + stats.element
        end
    elseif form.kind == "beast" then
        -- Gregar and Falzar Beast.
        if battle.emotion(side) == "full_synchro" then
            palette_add = NO_CHARGE_PALETTE
        end
    elseif arm and arm.palette_add then
        palette_add = arm.palette_add
    end
    attach(buster.arm, if arm then arm.anim else 0, palette_add)
end

-- What stops a shot's panel count (`byte_800FB4C`, by side): bodies,
-- neutral objects, blockers and reservations, and only the other side's
-- barrier bits.
local CLOSED = { [0] = 0x0D880080, [1] = 0x0E880080 }

-- `sub_800FAF6`: the ticks of recovery after a shot, by the Rapid stat and
-- the open panels from (x, y) forward, counting (x, y) itself, at most 5
-- (`data.rules.buster_recovery`).
function buster.recovery(navi: Object, x: number, y: number): number
    local closed = CLOSED[navi.alliance]
    local open = 0
    while true do
        local f = field.flags(x, y)
        if f == 0 or bit32.btest(f, closed) then
            break
        end
        open += 1
        x += navi.facing
    end
    local rapid = battle.navi(navi.alliance).rapid
    local ticks = data.rules.buster_recovery[rapid * 6 + math.min(open, 5)]
    if ticks == nil then
        error(string.format("Rapid %d reads past the buster's recovery table", rapid))
    end
    return ticks
end

-- The move that cuts a shot's recovery short: `object_canMove`, a held
-- direction (`sub_800FA54`) and a panel to step to (`sub_800F964`). Then
-- the arm and anything in the first related slot are let go, and the move
-- starts (`sub_80116AE`); true if it did.
function buster.move_cancel(navi: Object): boolean
    if not navi:can_move() then
        return false
    end
    local dir = navi:held_direction()
    if dir == 0 or navi:step_target(dir) == nil then
        return false
    end
    navi.related1 = nil
    navi.overlay = nil
    navi:exit_attack()
    navi:start_move(dir)
    return true
end

return buster

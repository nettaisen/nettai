--!strict
-- EXE4's deletion (a navi's action 2, 0x08010850; the role `actions.deletion`): while MegaMan's (NaviStats +0x23 = 0,
-- EXE4's content's one navi) and not in auto battle, drawn white each tick and by its phase (0x08010880):
--   0 (0x08010890): one fewer alive navi on the side (the count alone, 0x080079C6); its hit region, status visuals,
--     submersion and charge gone; no chip; the hurt animation (2); its reservation and barrier let go (0x08012DEC: its
--     second related and the barrier byte);
--   4 (0x080108CA), unless dimmed: its links let go (the first related, AIData +0x60), the deletion sound (0x6C) and two
--     explosions (effect row 3: one on the navi, one where the allocator's registers say), then 21 ticks;
--   8 (0x08010900): the 21 ticks;
--   0xC (0x08010914): 32 ticks of mosaic and fading, then the damage-carry record forgets it (0x08022218) and it goes,
--     its action kept.
-- (A navi in auto battle, or another navi, has its own deletion, 0x08010D4C: the engine runs its own for those.)

local deletion = {}

local DELETED = asset.sound("deleted")
-- Effect row 3 (0x080E1698: 14-03, animation 0, palette 0).
local EXPLOSION = new.effect { sprite = asset.sprite("deletion"), anim = 0, palette = 0 }

-- The phases.
local START, EXPLODE, WAIT, FADE = 0x0, 0x4, 0x8, 0xC

local function go(me: Object, phase: number)
    me.phase, me.phase_init = phase, 0
end

-- 0x08010890.
local function start(me: Object)
    me:drop_alive_count()
    me.collision:set_region(nil)
    me:drop_status_visuals()
    me:set_status_timer("submerged", 0)
    me:set_status("submerged", false)
    me:reset_charge()
    me:drop_chip()
    me.anim = 2
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    me.related2 = nil
    me:drop_barrier()
    go(me, EXPLODE)
end

-- 0x080108CA.
local function explode(me: Object)
    if battle.dimmed() then
        return
    end
    me.related1 = nil
    me.prevent_anim = 0
    me.barrier_visual = nil
    battle.play_sound(DELETED)
    local pos = me.pos
    battle.effect(pos, EXPLOSION)
    battle.effect_after_spawn(pos.z, EXPLOSION)
    me.timer = 21
    go(me, WAIT)
end

-- 0x08010900.
local function wait(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        go(me, FADE)
    end
end

-- 0x08010914: blocks of timer / 2 and an alpha of 16 less that, until 32.
local function fade(me: Object)
    local t = me.timer + 1
    me.timer = t
    local sprite = me.sprite
    if t ~= 32 then
        local half = bit32.rshift(t, 1)
        sprite.mosaic = if half ~= 0 then half else nil
        sprite.alpha = 16 - half
        return
    end
    sprite.mosaic = nil
    sprite.alpha = nil
    local side = me.alliance
    local carry = battle.damage_carry(side)
    if carry.target == me then
        carry.target = nil
        battle.set_damage_carry(side, carry)
    end
    me.visible = false
    me:set_lifecycle("destroy")
end

deletion.action = new.action {
    id = "deletion",
    update = function(me: Object)
        me.sprite.white = true
        local phase = me.phase
        if phase == START then
            start(me)
        elseif phase == EXPLODE then
            explode(me)
        elseif phase == WAIT then
            wait(me)
        elseif phase == FADE then
            fade(me)
        else
            error(string.format("a deletion's phase %#x reads past its table (0x08010880)", phase))
        end
    end,
}

return deletion

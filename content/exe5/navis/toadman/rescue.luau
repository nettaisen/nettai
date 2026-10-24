--!strict
-- ToadMan's rescue (EXE5 DS's party battle: 0x020048E8 and action 0x5A, 0x02004938; docs/design/exe5ds-map.md §4.7), the
-- roles' `revival`: when the navi on the field reaches 0 HP and ToadMan is a benched member of its party who may
-- support (rules/party), the navi isn't deleted: the battle dims, ToadMan's telop shows, three notes rise around the navi
-- and it is healed a quarter of its maximum HP, once a battle. Its steps (table 0x020CC39C), on the attack's step; the
-- sprite steps every tick (`sub_801BCD0`).
--
-- - 0 (0x0200494C): white every tick. It starts: the battle dims (battle flag 4), the navi's switch, the traps' and the
--   rescue's requests gone, it out of the trap's, a hover's and the support's states, out of the collision, the rescue
--   spent (the party block's +0xB), the screen dimmed; it lands on its destination panel (where it is, not on the
--   ground), out of a move, a guard, a flinch, a paralysis, a slide or a drag, asked no flinch; animation 2; its links
--   and overlay cut; its form's overlay restarted and told (its third parameter, 1). Once the screen is dimmed:
--   ToadMan's telop (HPRecov, the telop's sound).
-- - 4 (0x02004A48): white every tick. Once the telop is done, 60 ticks: the notes (effect 0x6C, the toad's note, by a
--   jitter of its place) at 20, 28 and 36; at 50 the heal (effect 6 and its sound), a quarter of the maximum HP (HP was
--   0), 120 ticks of flashing and animation 0. Then its statuses and anger end, its barrier goes, animation 0.
-- - 8 (0x02004B24): its form's overlay untold; the screen undims; then it is no longer revived, hovering or armed,
--   asked no switch or trap, the battle undims, its collision is back, and it idles.
-- (The HUD's parts and HP number, the bottom screen's button and the telop's banner by console are presentation the
-- engine draws its own way.)

local effects = require("../../lib/effects")
local hprecov = require("../../chips/hprecov").hprecov
local party = require("../../rules/party")
local regions = require("@exelib/regions")
local toadman = require("../toadman").toadman

local rescue = {}

export type State = {
    -- AIAttackVars+0x10: the step's ticks.
    timer: number,
}

local START, HEAL, END = 0x0, 0x4, 0x8
local FADE_SPEED = 4
-- The toad's note: the DS's effect 0x6C (0x02229C14's row: the GBA's sprite 10-44, animation 0).
local NOTE = new.effect { sprite = asset.sprite("toad-note"), anim = 0, palette = 0 }
-- The notes' ticks, the heal's, and the step's last.
local NOTES: { number } = { 0x14, 0x1C, 0x24 }
local HEALS_AT, ENDS_AT = 0x32, 0x3C
-- `AddRandomVarianceToTwoCoords`'s mask, and the notes' height (16 pixels up).
local JITTER, NOTE_Z = 0x1C, 0x10_0000
-- 0x8A.
local HEAL_SOUND = asset.sound("heal")
-- `sub_801A66C`'s flashing.
local FLASHING = 0x78

local function set_step(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- 0x0200494C
local function start(me: Object)
    me.sprite.white = true
    if me.step_init == 0 then
        battle.set_dimmed(true)
        for _, r in { "navi_switch", "anti_sword_triggered", "anti_damage_triggered", "revival" } :: { Request } do
            me:set_request(r, false)
        end
        me:set_navi_state("reviving", true)
        -- (0x0200496C: 0x408800.)
        for _, state in { "hovering", "trap_armed", "guarded" } :: { NaviStateFlag } do
            me:set_navi_state(state, false)
        end
        me.collision:set_region(nil)
        party.spend_rescue(me.alliance)
        battle.screen_fade("dim", FADE_SPEED)
        me:snap_to_future_panel()
        me:stop_moving()
        me:set_status("guard", false)
        me:set_request("flinch", false)
        me:set_animation(2)
        me.related1 = nil
        me.overlay = nil
        local part = me.related2
        if part then
            part.anim_loaded = 0xFF
        end
        me:overlay_stepping("told")
        me.step_init = 4
    end
    if battle.screen_fading() then
        return
    end
    dimming.chip_telop(me.alliance, hprecov)
    set_step(me, HEAL)
end

-- 0x02004A48
local function heal(me: Object, s: State)
    me.sprite.white = true
    if me.step_init == 0 then
        if not dimming.telop_done(me.alliance) then
            return
        end
        s.timer = 0
        me.step_init = 4
    end
    local t = s.timer
    if table.find(NOTES, t) then
        battle.effect(battle.jitter(JITTER, me.pos) + Vec3.new(0, 0, NOTE_Z), NOTE)
    elseif t == HEALS_AT then
        battle.effect(me.pos, effects.heal)
        battle.play_sound(HEAL_SOUND)
        me.hp = math.min(me.hp + bit32.rshift(me.max_hp, 2), me.max_hp)
        -- (Its HP number went at 0 HP; the DS's HUD draws its own. Here it shows again.)
        battle.show_navi_hp(me)
        me:set_status_timer("flash", FLASHING)
        me:set_status("flashing", true)
        me:set_animation(0)
    end
    t += 1
    s.timer = t
    if t == ENDS_AT then
        me:end_statuses()
        me:end_anger()
        me:drop_barrier()
        me.barrier_visual = nil
        me:set_animation(0)
        set_step(me, END)
    end
end

-- 0x02004B24
local function finish(me: Object)
    if me.step_init == 0 then
        me:overlay_stepping("normal")
        battle.screen_fade("undim", FADE_SPEED)
        me.step_init = 4
    end
    if battle.screen_fading() then
        return
    end
    for _, state in { "reviving", "hovering", "trap_armed" } :: { NaviStateFlag } do
        me:set_navi_state(state, false)
    end
    for _, r in { "navi_switch", "anti_sword_triggered", "anti_damage_triggered" } :: { Request } do
        me:set_request(r, false)
    end
    battle.set_dimmed(false)
    me.collision:set_region(regions.single)
    me:exit_attack()
end

-- 0x02004938
local function update(me: Object, s: State)
    local step = me.step
    if step == START then
        start(me)
    elseif step == HEAL then
        heal(me, s)
    elseif step == END then
        finish(me)
    else
        error(string.format("ToadMan's rescue's step %#x reads past its table (0x020CC39C)", step))
    end
    me:step_sprite()
end

rescue.action = new.action { id = "toadman/rescue", state = { timer = "u16" }, update = update }

-- 0x020048E8 (the rules' `navi_falling`): the rescue, when ToadMan can.
function rescue.navi_falling(side: number, navi: Object): boolean
    return party.rescue(side, navi, toadman)
end

return rescue

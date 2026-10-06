--!strict
-- What the ruleset starts and spawns by role (docs/design/content-model-v2.md
-- §7.4): each role names the definition that fills it.

local absorbed_obstacle = require("../objects/absorbed_obstacle")
local anti_recovery = require("../chips/antirecv/controller")
local antidmg_counter = require("../chips/antidmg/counter")
local antiswrd_counter = require("../chips/antiswrd/counter")
local barriers = require("../lib/barriers")
local beast_out = require("../chips/beastout")
local beat = require("../chips/beat")
local beast_rush = require("./beast/rush")
local bodygrd_counter = require("../chips/bodygrd/counter")
local charge_tackle = require("../navis/megaman/forms/chargecross/tackle")
local charged_shot = require("../navis/megaman/weapons/charged_shot")
local collision = require("./collision")
local dust_beast_scatter = require("../navis/megaman/forms/dustcross_beast/scatter")
local effects = require("../lib/effects")
local encased_bubble = require("../objects/encased_bubble/bubble")
local invalid = require("../chips/invalid")
local megabstr = require("../chips/megabstr")
local rush = require("../chips/rush")
local tango = require("../chips/tango")
local turn = require("../navis/megaman/turn")
local falling_rock = require("../objects/falling_rock")
local ground_drill = require("../navis/megaman/forms/groundcross/drill")
local regions = require("@exelib/regions")
local slash_beast_lunge = require("../navis/megaman/forms/slashcross_beast/lunge")
local sparks = require("../lib/sparks")
local stun_strike = require("../lib/swords/stun_strike")
local statuses = require("./status")
local supports = require("../lib/supports/controller")

return {
    actions = {
        -- The charged shot a navi's request starts from idle without its
        -- weapon's setup (the original's action 0x16).
        forced_charged_shot = charged_shot.action,
        -- The trap chips' counters, which a trap that caught a hit starts
        -- (the original's actions 0x47, 0x48 and 0x4B; reactive.rs).
        anti_damage_counter = antidmg_counter.action,
        anti_sword_counter = antiswrd_counter.action,
        body_guard_counter = bodygrd_counter.action,
        -- The strike the navi's request 0x80000 starts from idle (a
        -- machine sword's, by the attack's chip).
        stun_strike = stun_strike.action,
        -- What a switched-in navi protected at the battle's end runs
        -- (GroundCross's drill).
        switch_protect = ground_drill.action,
        -- The turn L or R starts where turning is enabled.
        turn = turn.action,
        -- A switched-in navi's knock-out (SlashCross Beast's lunge).
        switch_knockout = slash_beast_lunge.action,
        -- (`volley`, what the navi's volley request starts, isn't filled.
        -- The original starts its action 0x30, WideSht's and SuprSpr's
        -- routine, on whatever the attack's parameter bytes hold; no
        -- routine raises the request, and the port raises it nowhere.)
        -- The wrapper an attack runs inside while its `wrapped` is 1:
        -- the Beast Out rush (`sub_80EAD9C`).
        wrapper = beast_rush.action,
        -- DustCross Beast's scatter, during which the navi isn't grounded.
        dust_beast_scatter = dust_beast_scatter.action,
        -- ChargeCross's tackle, during which an invulnerable navi doesn't
        -- glow.
        charge_tackle = charge_tackle.action,
    },
    kinds = {
        -- What an obstacle absorbed flies off as.
        absorbed_obstacle = absorbed_obstacle.kind,
        -- GroundCross's charged chips' falling rocks.
        falling_rock = falling_rock.kind,
        -- The NaviCust supports' controller (Rush's, Beat's and Tango's).
        support = supports.kind,
        -- AntiRecv's counterattack, which a heal meets when the other side
        -- has AntiRecv armed.
        anti_recovery = anti_recovery.kind,
    },
    chips = {
        -- What a zeroed chip field reads (the original's chip 0): an empty
        -- hand's chip, and a chip use with none.
        zeroed = megabstr,
        -- The custom screen's Beast Out button, as a chip in the hand.
        beast_out = beast_out,
        -- What a selection the custom screen can't allow becomes.
        invalid = invalid,
        -- The chips the supports' telops name.
        rush = rush,
        beat = beat,
        tango = tango,
    },
    statuses = {
        -- What a damage word's paralysis bit (0x4000) makes its hits carry.
        damage_word_paralysis = statuses.paralyze_90,
        -- What a counter hit lands instead of a flinch.
        counter_paralysis = statuses.paralyze_150,
        -- What an aqua hit gives a body standing on ice.
        ice_freeze = statuses.freeze_150,
        -- The NaviCust on-hit bug's, by its level.
        hit_bug_blind = statuses.blind_1200,
        hit_bug_confuse = statuses.confuse_960,
    },
    effects = {
        -- A navi deleted (its two explosions), a cross merged into MegaMan,
        -- a navi arriving mid-battle or back from a Cross special, a form
        -- put on or taken off.
        deletion = effects.deletion,
        -- HP recovered: a heal's sparkle, a recovery chip's, the
        -- undershirt's credits'; a navi appearing mid-battle.
        recovery = effects.heal,
        -- The flash where a side cut in.
        cut_in_flash = effects.cut_in_flash,
        -- The mark over a navi whose trap sprang (AntiNavi, AntiRecv).
        trap_mark = effects.trap_mark,
        -- What an obstacle encased in ice or a bubble flickers with.
        encased = effects.explosion,
        -- The bursts around a navi going Beast Over.
        burst = effects.beast_over_burst,
    },
    sparks = {
        -- What a new collision registration's hits show until its owner
        -- names another.
        plain = sparks.plain,
        -- A blocked hit's.
        guard = sparks.guard,
        -- An eruption's hits' (a volcano panel's).
        eruption = sparks.fire,
        -- A thrown obstacle's landing's (`sub_80180EC`: the hit region's Param2 0, the plain spark).
        thrown_obstacle = sparks.plain,
        -- A navi's programs uninstalled.
        uninstall = sparks.uninstall,
    },
    regions = {
        -- A registration's own panel, which every setup gives it.
        anchor = regions.single,
    },
    collision = {
        -- A navi's body, its body when it floats, and what it reacts to.
        navi = collision.navi,
        floating_navi = collision.floating_navi,
        navi_target = collision.navi_target,
        -- An eruption (a volcano panel's), and what it reaches.
        eruption = collision.hitbox,
        eruption_target = collision.hits_both_sides,
        -- A thrown obstacle's landing, and what it reaches.
        thrown_obstacle = collision.breaking,
        thrown_obstacle_target = collision.hits_navis,
    },
    sounds = {
        -- A panel cracks or breaks; a panel turns to poison.
        panel_crack = asset.sound("panel-crack"),
        panel_poison = asset.sound("panel-poison"),
        -- A navi hit, as its own player hears it (and the custom-HP bug's
        -- damage) and as the other does (and an object hurt without
        -- flinching); an object taking damage; a blocked hit; a counter
        -- hit; a navi deleted.
        own_hit = asset.sound("own-hit"),
        hit = asset.sound("hit"),
        damage = asset.sound("damage"),
        guard = asset.sound("guard-hit"),
        counter_hit = asset.sound("counter-hit"),
        deleted = asset.sound("deleted"),
        -- HP recovered; a chip's damage bonus as it is used.
        recovery = asset.sound("heal"),
        damage_bonus = asset.sound("damage-bonus"),
        -- The battle pauses and resumes; the custom gauge fills; the
        -- low-HP alarm.
        pause = asset.sound("pause"),
        gauge_full = asset.sound("gauge-full"),
        low_hp = asset.sound("low-hp"),
        -- A side cuts in, a trap springs; a telop shows.
        cut_in = asset.sound("dimming-sparkle"),
        telop = asset.sound("telop"),
        -- The buster starts charging, and is charged.
        buster_charge = asset.sound("buster-charge"),
        buster_charged = asset.sound("buster-charged"),
        -- A navi frozen, bubbled, its bubble popped; confusion's stars.
        freeze = asset.sound("freeze"),
        bubble = asset.sound("bubble"),
        bubble_pop = asset.sound("bubble-pop"),
        confusion = asset.sound("confusion"),
        -- A navi turns invisible, shows again (or enters), arrives
        -- mid-battle; something goes (a canceled defensive chip,
        -- uninstalled programs, a Cross navi's special, a form taken off).
        invisible = asset.sound("invisible"),
        appear = asset.sound("appear"),
        arrive = asset.sound("point-appear"),
        fade = asset.sound("fade"),
        -- A thrown obstacle lifted, and flying.
        obstacle_lift = asset.sound("point-rise"),
        obstacle_throw = asset.sound("etomahwk"),
        -- A cross merging into MegaMan; a form change starting; a Cross
        -- put on (two sounds at once); a Beast Out put on; the beasts'
        -- roars; Beast Over's rumbles and bursts.
        burst = asset.sound("beast-over-burst"),
        -- SELECT with too little gauge: its player hears that it can't (and
        -- what can't be picked or taken back on the custom screen).
        refused = asset.sound("cant-jack-in"),
        -- The custom screen's, which its player hears: the window slides in;
        -- the cursor moves; SELECT hides and shows the window; a pick, OK,
        -- a pick taken back; the Cross window opens and closes, a Cross is
        -- put on; L's no-running message; R's description opens and closes;
        -- Beast Out chosen (two sounds, the first by the console's version:
        -- Falzar's, Gregar's); a Beast Out or Cross taken back;
        -- ChpShufl's re-deal and its shuffles; DustCross's scrap, a chip at a
        -- time and when it is done; the Program Advance animation's chips of
        -- the recipe and the Program Advance.
        custom_open = asset.sound("select-79"),
        custom_cursor = asset.sound("menu-cur-move"),
        custom_hide = asset.sound("cur-move-80"),
        custom_pick = asset.sound("menu-select"),
        custom_ok = asset.sound("select-82"),
        custom_back = asset.sound("exit-submenu"),
        custom_cross_open = asset.sound("select-7a"),
        custom_cross_close = asset.sound("cross-window-close"),
        custom_cross_chosen = asset.sound("delta-ray-2"),
        custom_run_message = asset.sound("unselect-7b"),
        custom_description = asset.sound("description-open"),
        custom_description_close = asset.sound("description-close"),
        custom_beast_out_falzar = asset.sound("beast-out-chosen-falzar"),
        custom_beast_out_gregar = asset.sound("beast-out-chosen-gregar"),
        custom_beast_out_flash = asset.sound("blinding-flash"),
        custom_cancel = asset.sound("custom-cancel"),
        custom_redeal = asset.sound("elmnt-man-2"),
        custom_redeal_shuffle = asset.sound("hop"),
        custom_scrap = asset.sound("blizzard-grow"),
        custom_scrap_done = asset.sound("elmnt-man-2"),
        program_advance_part = asset.sound("err-select-91"),
        program_advance = asset.sound("delta-ray-2"),
    },
    music = {
        -- A link battle's, whatever its stage's.
        link_battle = asset.sound("virus-battle"),
        -- The winner's (in special battles, and otherwise) and the loser's.
        winner_special = asset.sound("winner-0"),
        winner = asset.sound("winner-1"),
        loser = asset.sound("loser"),
    },
    sprites = {
        -- The charge glow of a B charge and of an A charge; the Full
        -- Synchro aura.
        charge_glow = asset.sprite("charge-glow"),
        charge_glow_a = asset.sprite("charge-glow-a"),
        full_synchro_aura = asset.sprite("full-synchro-aura"),
        -- The status visuals over a navi (confusion's stars, blindness, the
        -- immobilized one), the ice block around a frozen navi and the
        -- bubble around a bubbled one.
        confusion = asset.sprite("confusion"),
        blindness = asset.sprite("blindness"),
        immobilized = asset.sprite("rock-cubes"),
        ice = asset.sprite("ice-block"),
        bubble = asset.sprite("bubble"),
        -- The mark over a navi a hit told something about ("!!", the HP
        -- bug's).
        hit_marker = asset.sprite("hit-marker"),
        -- A volcano panel's eruption; the Beast forms' lock-on marker.
        eruption = asset.sprite("eruption"),
        target_marker = asset.sprite("lockon-marker"),
    },
    banners = {
        -- A round's start (the battle's number), a turn's, the final turns'.
        round_start = asset.banner("battle-number-start"),
        turn_start = asset.banner("turn-start"),
        final_turn = asset.banner("final-turn"),
        -- A draw; a win's where it isn't the winner's navi's (outside a link battle: `sub_80081A4`), and a win
        -- and a loss on the judge's ruling (`sub_800825A`); the damage judge's.
        draw = asset.banner("draw"),
        win = asset.banner("enemy-deleted"),
        win_judged = asset.banner("you-win"),
        lose_judged = asset.banner("you-lose"),
        judge = asset.banner("hit-damage-judge"),
        -- A telop on the local player's side, and on the other's.
        telop = asset.banner("telop"),
        telop_remote = asset.banner("telop-remote"),
        -- A Program Advance, and a selection that makes none.
        program_advance = asset.banner("program-advance"),
        program_advance_empty = asset.banner("program-advance-empty"),
    },
    hooks = {
        -- The first barrier (`sub_8013892`): the navi enters with the
        -- barrier its stat names (the NaviCust's FirstBarrier sets the
        -- Barrier chip's; the patch cards Barr100's, Barr200's, BblWrap's
        -- bubble or LifeAur's aura), and its visual.
        first_barrier = function(navi: Object)
            local barrier = battle.navi(navi.alliance).first_barrier
            if barrier then
                barriers.raise(navi, barrier, true)
            end
        end,
        -- What an obstacle encased in ice or a bubble becomes
        -- (`sub_801813A`'s end).
        encased = encased_bubble.replace,
    },
}

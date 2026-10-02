--!strict
-- What the ruleset starts and spawns by role (docs/design/content-model-v2.md
-- §7.4): each role names the definition that fills it.

local absorbed_obstacle = require("../objects/absorbed-obstacle/absorbed_obstacle")
local anti_recovery = require("../chips/antirecv/controller")
local antidmg_counter = require("../chips/antidmg/counter")
local antiswrd_counter = require("../chips/antiswrd/counter")
local barriers = require("../lib/barriers/barriers")
local beast_out = require("../chips/beastout/chip")
local beat = require("../chips/beat/chip")
local beast_claw = require("../navis/megaman/weapons/beast-claw/weapon")
local bodygrd_counter = require("../chips/bodygrd/counter")
local charge_tackle = require("../navis/megaman/forms/chargecross/tackle")
local charged_shot = require("../navis/megaman/weapons/charged-shot/weapon")
local collision = require("./collision")
local dust_beast_scatter = require("../navis/megaman/forms/dustcross-beast/scatter")
local effects = require("../lib/effects")
local encased_bubble = require("../objects/encased-bubble/bubble")
local invalid = require("../chips/invalid/chip")
local megabstr = require("../chips/megabstr/chip")
local rush = require("../chips/rush/chip")
local tango = require("../chips/tango/chip")
local turn = require("../navis/megaman/turn")
local falling_rock = require("../objects/falling-rock/falling_rock")
local ground_drill = require("../navis/megaman/forms/groundcross/drill")
local lockon = require("./lockon")
local regions = require("../lib/regions")
local slash_beast_lunge = require("../navis/megaman/forms/slashcross-beast/lunge")
local slash_charge = require("../navis/megaman/forms/slashcross/charge")
local sparks = require("../lib/sparks")
local stun_strike = require("../lib/swords/stun_strike")
local statuses = require("./status")
local supports = require("../lib/supports/controller")

return define.roles {
    id = "bn6:roles",
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
        -- What a Cross navi protected at the battle's end runs
        -- (GroundCross's drill).
        cross_protect = ground_drill.action,
        -- The turn L or R starts where turning is enabled.
        turn = turn.action,
        -- A Cross navi's knock-out (SlashCross Beast's lunge).
        cross_death = slash_beast_lunge.action,
        -- (`volley`, what the navi's volley request starts, isn't filled.
        -- The original starts its action 0x30, WideSht's and SuprSpr's
        -- routine, on whatever the attack's parameter bytes hold; no
        -- routine raises the request, and the port raises it nowhere.)
        -- SlashCross's charged slash, which the Beast rush recognizes.
        charged_sword = slash_charge.action,
        -- The Beast forms' charged claw.
        beast_claw = beast_claw.action,
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
    lockon = {
        -- The Beast forms' charged claw's lock-on mode, whatever chip its
        -- attack carries.
        beast_claw = lockon.beast_claw,
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
        beast_over_burst = effects.beast_over_burst,
    },
    sparks = {
        -- What a new collision registration's hits show until its owner
        -- names another.
        plain = sparks.plain,
        -- A blocked hit's.
        guard = sparks.guard,
        -- An eruption's hits' (a volcano panel's).
        eruption = sparks.fire,
        -- A thrown obstacle's landing's.
        thrown_obstacle = sparks.charged,
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
        panel_crack = asset.sound("bn6:panel-crack"),
        panel_poison = asset.sound("bn6:panel-poison"),
        -- A navi hit, as its own player hears it (and the custom-HP bug's
        -- damage) and as the other does (and an object hurt without
        -- flinching); an object taking damage; a blocked hit; a counter
        -- hit; a navi deleted.
        own_hit = asset.sound("bn6:own-hit"),
        hit = asset.sound("bn6:hit"),
        damage = asset.sound("bn6:damage"),
        guard = asset.sound("bn6:guard-hit"),
        counter_hit = asset.sound("bn6:counter-hit"),
        deleted = asset.sound("bn6:deleted"),
        -- HP recovered; a chip's damage bonus as it is used.
        recovery = asset.sound("bn6:heal"),
        damage_bonus = asset.sound("bn6:damage-bonus"),
        -- The battle pauses and resumes; the custom gauge fills; the
        -- low-HP alarm.
        pause = asset.sound("bn6:pause"),
        gauge_full = asset.sound("bn6:gauge-full"),
        low_hp = asset.sound("bn6:low-hp"),
        -- A side cuts in, a trap springs; a telop shows.
        cut_in = asset.sound("bn6:dimming-sparkle"),
        telop = asset.sound("bn6:telop"),
        -- The buster starts charging, and is charged.
        buster_charge = asset.sound("bn6:buster-charge"),
        buster_charged = asset.sound("bn6:buster-charged"),
        -- A navi frozen, bubbled, its bubble popped; confusion's stars.
        freeze = asset.sound("bn6:freeze"),
        bubble = asset.sound("bn6:bubble"),
        bubble_pop = asset.sound("bn6:bubble-pop"),
        confusion = asset.sound("bn6:confusion"),
        -- A navi turns invisible, shows again (or enters), arrives
        -- mid-battle; something goes (a cancelled defensive chip,
        -- uninstalled programs, a Cross navi's special, a form taken off).
        invisible = asset.sound("bn6:invisible"),
        appear = asset.sound("bn6:appear"),
        arrive = asset.sound("bn6:point-appear"),
        fade = asset.sound("bn6:fade"),
        -- A thrown obstacle lifted, and flying.
        obstacle_lift = asset.sound("bn6:point-rise"),
        obstacle_throw = asset.sound("bn6:etomahwk"),
        -- A cross merging into MegaMan; a form change starting; a Cross
        -- put on (two sounds at once); a Beast Out put on; the beasts'
        -- roars; Beast Over's rumbles and bursts.
        beast_over_burst = asset.sound("bn6:beast-over-burst"),
        -- A Cross navi starting its special's chip.
        cross_special = asset.sound("bn6:elmnt-man-2"),
        -- SELECT with too little gauge: its player hears that it can't (and
        -- what can't be picked or taken back on the custom screen).
        refused = asset.sound("bn6:cant-jack-in"),
        -- The custom screen's, which its player hears: the window slides in;
        -- the cursor moves; SELECT hides and shows the window; a pick, OK,
        -- a pick taken back; the Cross window opens and closes, a Cross is
        -- put on; L's no-running message; R's description opens and closes;
        -- Beast Out chosen (two sounds, the first by the console's version:
        -- Falzar's, Gregar's); a Beast Out or Cross taken back;
        -- ChpShufl's re-deal and its shuffles; DustCross's scrap, a chip at a
        -- time and when it is done; the Program Advance animation's chips of
        -- the recipe and the Program Advance.
        custom_open = asset.sound("bn6:select-79"),
        custom_cursor = asset.sound("bn6:menu-cur-move"),
        custom_hide = asset.sound("bn6:cur-move-80"),
        custom_pick = asset.sound("bn6:menu-select"),
        custom_ok = asset.sound("bn6:select-82"),
        custom_back = asset.sound("bn6:exit-submenu"),
        custom_cross_open = asset.sound("bn6:select-7a"),
        custom_cross_close = asset.sound("bn6:cross-window-close"),
        custom_cross_chosen = asset.sound("bn6:delta-ray-2"),
        custom_run_message = asset.sound("bn6:unselect-7b"),
        custom_description = asset.sound("bn6:description-open"),
        custom_description_close = asset.sound("bn6:description-close"),
        custom_beast_out_falzar = asset.sound("bn6:beast-out-chosen-falzar"),
        custom_beast_out_gregar = asset.sound("bn6:beast-out-chosen-gregar"),
        custom_beast_out_flash = asset.sound("bn6:blinding-flash"),
        custom_cancel = asset.sound("bn6:custom-cancel"),
        custom_redeal = asset.sound("bn6:elmnt-man-2"),
        custom_redeal_shuffle = asset.sound("bn6:hop"),
        custom_scrap = asset.sound("bn6:blizzard-grow"),
        custom_scrap_done = asset.sound("bn6:elmnt-man-2"),
        program_advance_part = asset.sound("bn6:err-select-91"),
        program_advance = asset.sound("bn6:delta-ray-2"),
    },
    music = {
        -- A link battle's, whatever its stage's.
        link_battle = asset.sound("bn6:virus-battle"),
        -- The winner's (in special battles, and otherwise) and the loser's.
        winner_special = asset.sound("bn6:winner-0"),
        winner = asset.sound("bn6:winner-1"),
        loser = asset.sound("bn6:loser"),
    },
    sprites = {
        -- The charge glow of a B charge and of an A charge; the Full
        -- Synchro aura.
        charge_glow = asset.sprite("bn6:charge-glow"),
        charge_glow_a = asset.sprite("bn6:charge-glow-a"),
        full_synchro_aura = asset.sprite("bn6:full-synchro-aura"),
        -- The status visuals over a navi (confusion's stars, blindness, the
        -- immobilized one), the ice block around a frozen navi and the
        -- bubble around a bubbled one.
        confusion = asset.sprite("bn6:confusion"),
        blindness = asset.sprite("bn6:blindness"),
        immobilized = asset.sprite("bn6:rock-cubes"),
        ice = asset.sprite("bn6:ice-block"),
        bubble = asset.sprite("bn6:bubble"),
        -- The mark over a navi a hit told something about ("!!", the HP
        -- bug's).
        hit_marker = asset.sprite("bn6:hit-marker"),
        -- A volcano panel's eruption; the Beast forms' lock-on marker.
        eruption = asset.sprite("bn6:eruption"),
        lockon_marker = asset.sprite("bn6:lockon-marker"),
    },
    banners = {
        -- A round's start (the battle's number), a turn's, the final turns'.
        round_start = asset.banner("bn6:battle-number-start"),
        turn_start = asset.banner("bn6:turn-start"),
        final_turn = asset.banner("bn6:final-turn"),
        -- A draw; the damage judge's.
        draw = asset.banner("bn6:draw"),
        judge = asset.banner("bn6:hit-damage-judge"),
        -- A telop on the local player's side, and on the other's.
        telop = asset.banner("bn6:telop"),
        telop_remote = asset.banner("bn6:telop-remote"),
        -- A Program Advance, and a selection that makes none.
        program_advance = asset.banner("bn6:program-advance"),
        program_advance_empty = asset.banner("bn6:program-advance-empty"),
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

import { Image, signal, Text, token, View } from "nsh";
import {
  clearEntitlementToken,
  connectCloudSession,
  copyToClipboard,
  deleteCustomPet,
  generateEncryptedArchive,
  generateJsonlExport,
  generateParentsPeaceLetter,
  generatePetWeeklyLetter,
  getCloudEntitlements,
  getDewSummary,
  getDisclaimerText,
  getEncryptedArchivePreview,
  getHearthGiftCount,
  getJsonlExportPreview,
  getMeSubTab,
  getNoticeText,
  getRestoringPurchases,
  getRestoreStatusMessage,
  getState,
  getVisualTheme,
  type MeSubTab,
  pickCustomPetPhotoDataUrl,
  pullWebdavSync,
  purchaseHearthGiftPass,
  restoreApplePurchases,
  restoreFromRecoveryPhrase,
  runE2eeSyncNow,
  sendMorningTestNotification,
  setActiveCustomPet,
  setMeSubTab,
  setShowAtelierModal,
  setShowCompanionLetterModal,
  setShowQaStudio,
  setVisualTheme,
  toggleMorningPush,
  updateAiConfig,
  updateCloudUrlAndHandle,
  updateSyncConfig,
  upsertCustomPet,
  verifyEntitlements,
  wipeVaultAndReset,
} from "../store";
import {
  ActionButton,
  Card,
  ChoiceCard,
  CollapsibleCard,
  FieldInput,
  Hairline,
  SectionHeader,
  SegmentedTabs,
  SelectChip,
  SheetDialog,
  SunkenBox,
  SwitchRow,
  VitalBadge,
} from "../ui/components";
import { QiIcon } from "../ui/icons";
import { tr } from "../ui/i18n";
import { ThemeSwitcherStrip } from "../ui/sanctuary";

export function MeVaultPage() {
  // Recovery / export / legal / wipe modals
  const showRecoveryPhrase = signal(false);
  const showRestoreSheet = signal(false);
  const restorePhraseInput = signal("");
  const restoreSourceInput = signal("");
  const restoreMode = signal<"icloud_drive" | "byo_server" | "archive">("icloud_drive");
  const showNoticeSheet = signal(false);
  const showPrivacySheet = signal(false);
  const showWipeConfirm = signal(false);
  const cloudAccountOpen = signal(false);

  // Custom Pet Companion Studio form state
  const editPetId = signal<string | null>(null);
  const draftPetName = signal("");
  const draftSpecies = signal<"cat" | "dog" | "rabbit" | "fox" | "other">("cat");
  const draftPersonaKey = signal<"calm" | "playful" | "wise" | "clingy">("calm");
  const draftPersonaNote = signal("");
  const draftPhotoDataUrl = signal("");
  const draftFrame = signal<
    "celadon_oval" | "lacquer_arch" | "paper_cut" | "starlight_halo" | "warm_stamp"
  >("celadon_oval");
  const draftStarlight = signal(false);
  const draftPawStamp = signal("");

  const state = () => getState();
  const settings = () => state().settings;
  const entitlements = () => state().entitlements;
  const solar = () => state().solar_atelier;
  const ent = () => getCloudEntitlements();
  const petQuota = () => solar().custom_pet_quota;
  const customPets = () => solar().custom_pets;
  const activeCustomPet = () => solar().active_custom_pet;
  const petLetters = () => solar().pet_weekly_letters;

  const subTabs = (): { id: MeSubTab; label: string }[] => [
    { id: "pet_studio", label: tr("me.subtab_pet_studio") },
    { id: "atelier_billing", label: tr("me.subtab_atelier") },
    { id: "vault_export", label: tr("me.subtab_vault") },
  ];

  const syncModes = () => [
    { id: "icloud_drive", label: tr("me.icloud_title") },
    { id: "webdav", label: tr("me.webdav_title") },
    { id: "byo_server", label: tr("me.byo_server_label") },
    { id: "local_only", label: tr("common.local_only") },
  ];

  const aiModes = () => [
    { id: "official_zdr_cloud", label: tr("me.ai_mode_zdr") },
    { id: "on_device_local", label: tr("me.ai_mode_local") },
    { id: "lan_ollama", label: tr("me.ai_mode_ollama") },
  ];

  const speciesOptions = () => [
    { id: "cat", label: tr("pet.species_cat") },
    { id: "dog", label: tr("pet.species_dog") },
    { id: "rabbit", label: tr("pet.species_rabbit") },
    { id: "fox", label: tr("pet.species_fox") },
    { id: "other", label: tr("pet.species_other") },
  ];

  const personaOptions = () => [
    { id: "calm", label: tr("pet.personality_calm") },
    { id: "playful", label: tr("pet.personality_playful") },
    { id: "wise", label: tr("pet.personality_wise") },
    { id: "clingy", label: tr("pet.personality_clingy") },
  ];

  const frameOptions = () => [
    { id: "celadon_oval", label: tr("pet.frame_celadon_oval") },
    { id: "lacquer_arch", label: tr("pet.frame_lacquer_arch") },
    { id: "paper_cut", label: tr("pet.frame_paper_cut") },
    { id: "starlight_halo", label: tr("pet.frame_starlight_halo") },
    { id: "warm_stamp", label: tr("pet.frame_warm_stamp") },
  ];

  const resetPetDraft = () => {
    editPetId.set(null);
    draftPetName.set("");
    draftSpecies.set("cat");
    draftPersonaKey.set("calm");
    draftPersonaNote.set("");
    draftPhotoDataUrl.set("");
    draftFrame.set("celadon_oval");
    draftStarlight.set(false);
    draftPawStamp.set("");
  };

  const loadPetIntoEditor = (petId: string) => {
    const found = customPets().find((p) => p.id === petId);
    if (!found) return;
    editPetId.set(found.id);
    draftPetName.set(found.name);
    draftSpecies.set(
      (found.species === "dog" || found.species === "rabbit" || found.species === "fox" || found.species === "other"
        ? found.species
        : "cat") as "cat" | "dog" | "rabbit" | "fox" | "other",
    );
    draftPersonaNote.set(found.persona);
    draftPhotoDataUrl.set(found.photo_data_url);
    draftFrame.set(
      (found.die_cut_frame === "lacquer_arch" ||
      found.die_cut_frame === "paper_cut" ||
      found.die_cut_frame === "starlight_halo" ||
      found.die_cut_frame === "warm_stamp"
        ? found.die_cut_frame
        : "celadon_oval") as
        | "celadon_oval"
        | "lacquer_arch"
        | "paper_cut"
        | "starlight_halo"
        | "warm_stamp",
    );
    draftStarlight.set(found.starlight_memorial);
    draftPawStamp.set(found.paw_stamp_title);
  };

  const submitPetForm = () => {
    const personaLabel =
      draftPersonaNote().trim() ||
      personaOptions().find((p) => p.id === draftPersonaKey())?.label ||
      tr("pet.personality_calm");
    const saved = upsertCustomPet({
      name: draftPetName(),
      species: draftSpecies(),
      persona: personaLabel,
      photoDataUrl: draftPhotoDataUrl(),
      dieCutFrame: draftFrame(),
      starlightMemorial: draftStarlight(),
      pawStampTitle: petQuota().is_pro_ex_libris_unlocked ? draftPawStamp() : "",
      isActive: true,
      petId: editPetId(),
    });
    if (saved) {
      resetPetDraft();
    }
  };

  const handleUploadPetPhoto = async () => {
    const dataUrl = await pickCustomPetPhotoDataUrl();
    if (dataUrl) {
      draftPhotoDataUrl.set(dataUrl);
    }
  };

  const formatPetSpecies = (species: string) => {
    switch (species) {
      case "dog":
        return tr("pet.species_dog");
      case "rabbit":
        return tr("pet.species_rabbit");
      case "fox":
        return tr("pet.species_fox");
      case "other":
        return tr("pet.species_other");
      default:
        return tr("pet.species_cat");
    }
  };

  const formatFrameLabel = (frame: string) => {
    switch (frame) {
      case "lacquer_arch":
        return tr("pet.frame_lacquer_arch");
      case "paper_cut":
        return tr("pet.frame_paper_cut");
      case "starlight_halo":
        return tr("pet.frame_starlight_halo");
      case "warm_stamp":
        return tr("pet.frame_warm_stamp");
      default:
        return tr("pet.frame_celadon_oval");
    }
  };

  const targetPetForLetter = () => activeCustomPet() ?? customPets()[0] ?? null;

  return (
    <View class="gap-4">
      <SegmentedTabs
        items={subTabs()}
        activeId={getMeSubTab()}
        onSelect={(id) => setMeSubTab(id)}
      />

      {/* Subview 1: Sanctuary & Custom Pet Studio */}
      <If when={getMeSubTab() === "pet_studio"}>
        <View class="gap-4">
          <Card>
            <SectionHeader
              title={tr("me.theme_section")}
              subtitle={tr("themes.switcher_subtitle")}
              right={<VitalBadge tone="prime_ready" label={`${getDewSummary().points} 💧`} />}
            />
            <ThemeSwitcherStrip
              activeId={getVisualTheme()}
              detailed={true}
              onSelect={(id) => setVisualTheme(id)}
            />
          </Card>

          {/* Custom Pet Companion Studio */}
          <Card>
            <SectionHeader
              title={tr("pet.studio_title")}
              subtitle={tr("pet.studio_subtitle")}
              right={
                <VitalBadge
                  tone={petQuota().can_add_more ? "prime_ready" : "neutral"}
                  label={tr("pet.slots_used", {
                    used: petQuota().used_count,
                    limit: petQuota().allowed_count,
                  })}
                />
              }
            />

            <SunkenBox>
              <View class="flex-row flex-wrap gap-1.5">
                <VitalBadge tone="prime_ready" label={tr("atelier.free_forever_pill")} />
                <VitalBadge tone="neutral" label={tr("atelier.pro_pets_pill")} />
              </View>
              <Text class="text-xs leading-relaxed text-muted-foreground">
                {entitlements().is_atelier_unlocked
                  ? tr("pet.pro_slots_limit")
                  : tr("pet.free_slot_forever")}
              </Text>
              <If
                when={
                  entitlements().state === "expired_read_only_vault" ||
                  petQuota().retained_above_free_quota
                }
              >
                <Text class="text-xs font-medium text-vital-prime">
                  {tr("pet.vault_preserved_notice")}
                </Text>
              </If>
            </SunkenBox>

            {/* Saved custom pets & seasonal companion preset switcher */}
            <View class="gap-2">
              <View class="flex-row flex-wrap items-center justify-between gap-2">
                <SelectChip
                  label={tr("pet.preset_xiaoman")}
                  selected={activeCustomPet() === null}
                  onPress={() => setActiveCustomPet("")}
                />
                <If when={Boolean(editPetId())}>
                  <ActionButton
                    title={tr("common.add")}
                    size="sm"
                    variant="ghost"
                    onPress={() => resetPetDraft()}
                  />
                </If>
              </View>

              <If when={customPets().length > 0}>
                <View class="gap-2.5">
                  <Each of={customPets()} key={(pet) => pet.id}>
                    {(pet) => (
                      <SunkenBox>
                        <View class="flex-row items-center gap-3">
                          <View class="size-12 items-center justify-center overflow-hidden rounded-full border border-border bg-card">
                            <If
                              when={Boolean(pet().photo_data_url)}
                              else={
                                <QiIcon
                                  name="custom_pet"
                                  size={22}
                                  color={
                                    pet().starlight_memorial
                                      ? token("ink-gold")
                                      : token("stamp-ink")
                                  }
                                />
                              }
                            >
                              <Image
                                src={pet().photo_data_url}
                                class="size-full object-cover"
                                accessibilityLabel={pet().name}
                              />
                            </If>
                          </View>

                          <View class="flex-1 gap-0.5">
                            <View class="flex-row flex-wrap items-center gap-1.5">
                              <Text class="text-sm font-bold text-foreground">{pet().name}</Text>
                              <VitalBadge
                                tone={pet().is_active ? "prime_ready" : "neutral"}
                                label={
                                  pet().is_active ? tr("pet.active_pet") : formatPetSpecies(pet().species)
                                }
                              />
                              <If when={pet().starlight_memorial}>
                                <VitalBadge tone="moderate_steady" label={tr("pet.memorial_badge")} />
                              </If>
                            </View>
                            <Text class="text-xs text-muted-foreground">
                              {`${formatFrameLabel(pet().die_cut_frame)} · ${pet().persona}`}
                            </Text>
                            <If when={Boolean(pet().paw_stamp_title)}>
                              <Text class="text-[11px] font-medium text-wax-seal">
                                {`${tr("pet.paw_ex_libris_title")}: ${pet().paw_stamp_title}`}
                              </Text>
                            </If>
                          </View>
                        </View>

                        <View class="flex-row flex-wrap gap-2 pt-1">
                          <ActionButton
                            title={pet().is_active ? tr("common.active") : tr("pet.set_active")}
                            size="sm"
                            variant={pet().is_active ? "secondary" : "primary"}
                            disabled={pet().is_active}
                            onPress={() => setActiveCustomPet(pet().id, pet().name)}
                          />
                          <ActionButton
                            title={tr("common.edit")}
                            size="sm"
                            variant="outline"
                            onPress={() => loadPetIntoEditor(pet().id)}
                          />
                          <ActionButton
                            title={tr("pet.delete_pet")}
                            size="sm"
                            variant="ghost"
                            onPress={() => {
                              if (editPetId() === pet().id) resetPetDraft();
                              deleteCustomPet(pet().id, pet().name);
                            }}
                          />
                        </View>
                      </SunkenBox>
                    )}
                  </Each>
                </View>
              </If>
            </View>

            <Hairline />

            {/* Create or edit pet form */}
            <View class="gap-3">
              <Text role="header" class="text-sm font-bold text-foreground">
                {editPetId() ? tr("common.edit") : tr("pet.add_pet")}
              </Text>

              <If when={!editPetId() && !petQuota().can_add_more}>
                <SunkenBox>
                  <Text class="text-xs leading-relaxed text-muted-foreground">
                    {tr("pet.slot_limit_reached", { limit: petQuota().allowed_count })}
                  </Text>
                  <ActionButton
                    title={tr("today.open_atelier")}
                    icon="sparkles"
                    size="sm"
                    variant="outline"
                    onPress={() => setShowAtelierModal(true)}
                  />
                </SunkenBox>
              </If>

              <View class="flex-row flex-wrap items-center gap-3">
                <ActionButton
                  title={tr("pet.upload_photo")}
                  icon="camera"
                  size="sm"
                  variant="outline"
                  onPress={() => void handleUploadPetPhoto()}
                />
                <If when={Boolean(draftPhotoDataUrl())}>
                  <ActionButton
                    title={tr("common.remove")}
                    size="sm"
                    variant="ghost"
                    onPress={() => draftPhotoDataUrl.set("")}
                  />
                </If>
              </View>
              <Text class="text-[11px] text-muted-foreground">{tr("pet.photo_tip")}</Text>

              <View class="gap-1.5">
                <Text class="text-xs font-semibold text-foreground">{tr("pet.rename_pet")}</Text>
                <FieldInput
                  value={draftPetName()}
                  placeholder={tr("pet.name_placeholder")}
                  onChange={(v) => draftPetName.set(v)}
                />
              </View>

              <View class="gap-1.5">
                <Text class="text-xs font-semibold text-foreground">{tr("pet.species_label")}</Text>
                <View class="flex-row flex-wrap gap-1.5">
                  <Each of={speciesOptions()} key={(s) => s.id}>
                    {(s) => (
                      <SelectChip
                        label={s().label}
                        selected={draftSpecies() === s().id}
                        onPress={() =>
                          draftSpecies.set(s().id as "cat" | "dog" | "rabbit" | "fox" | "other")
                        }
                      />
                    )}
                  </Each>
                </View>
              </View>

              <View class="gap-1.5">
                <Text class="text-xs font-semibold text-foreground">{tr("pet.personality")}</Text>
                <View class="flex-row flex-wrap gap-1.5">
                  <Each of={personaOptions()} key={(p) => p.id}>
                    {(p) => (
                      <SelectChip
                        label={p().label}
                        selected={draftPersonaKey() === p().id}
                        onPress={() => {
                          draftPersonaKey.set(p().id as "calm" | "playful" | "wise" | "clingy");
                          draftPersonaNote.set(p().label);
                        }}
                      />
                    )}
                  </Each>
                </View>
                <FieldInput
                  value={draftPersonaNote()}
                  placeholder={tr("pet.memorial_tribute_placeholder")}
                  onChange={(v) => draftPersonaNote.set(v)}
                />
              </View>

              <View class="gap-1.5">
                <Text class="text-xs font-semibold text-foreground">{tr("pet.frame_label")}</Text>
                <View class="flex-row flex-wrap gap-1.5">
                  <Each of={frameOptions()} key={(f) => f.id}>
                    {(f) => (
                      <SelectChip
                        label={f().label}
                        selected={draftFrame() === f().id}
                        onPress={() =>
                          draftFrame.set(
                            f().id as
                              | "celadon_oval"
                              | "lacquer_arch"
                              | "paper_cut"
                              | "starlight_halo"
                              | "warm_stamp",
                          )
                        }
                      />
                    )}
                  </Each>
                </View>
              </View>

              <ChoiceCard
                title={tr("pet.memorial_toggle")}
                subtitle={tr("pet.memorial_gentle_aura")}
                icon="starlight"
                selected={draftStarlight()}
                onPress={() => draftStarlight.set(!draftStarlight())}
              />

              <SunkenBox>
                <View class="flex-row items-center justify-between gap-2">
                  <Text class="text-xs font-semibold text-foreground">
                    {tr("pet.paw_ex_libris_title")}
                  </Text>
                  <VitalBadge
                    tone={petQuota().is_pro_ex_libris_unlocked ? "prime_ready" : "neutral"}
                    label={
                      petQuota().is_pro_ex_libris_unlocked
                        ? tr("common.unlocked")
                        : tr("common.pro")
                    }
                  />
                </View>
                <Text class="text-[11px] leading-relaxed text-muted-foreground">
                  {tr("pet.paw_ex_libris_desc")}
                </Text>
                <FieldInput
                  value={draftPawStamp()}
                  placeholder={tr("pet.paw_seal_placeholder")}
                  onChange={(v) => draftPawStamp.set(v)}
                />
              </SunkenBox>

              <ActionButton
                title={editPetId() ? tr("common.save") : tr("pet.save_pet")}
                icon="custom_pet"
                onPress={submitPetForm}
              />
            </View>
          </Card>

          {/* First-person custom pet weekly letters */}
          <Card>
            <SectionHeader
              title={tr("pet.weekly_letter_title")}
              subtitle={tr("pet.weekly_letter_pro_lock")}
              right={<VitalBadge tone="neutral" label={tr("pet.weekly_letter_badge")} />}
            />

            <If when={targetPetForLetter() !== null}>
              <View class="flex-row flex-wrap gap-2">
                <If
                  when={petQuota().can_generate_weekly_letter}
                  else={
                    <ActionButton
                      title={tr("today.open_atelier")}
                      icon="sparkles"
                      size="sm"
                      variant="outline"
                      onPress={() => setShowAtelierModal(true)}
                    />
                  }
                >
                  <ActionButton
                    title={tr("pet.weekly_letter_generate")}
                    icon="envelope"
                    size="sm"
                    onPress={() => {
                      const target = targetPetForLetter();
                      if (target) generatePetWeeklyLetter(target.id, target.name);
                    }}
                  />
                </If>
                <If when={petLetters().length > 0}>
                  <ActionButton
                    title={tr("today.open_letter")}
                    icon="wax_seal"
                    size="sm"
                    variant="secondary"
                    onPress={() => setShowCompanionLetterModal(true)}
                  />
                </If>
              </View>
            </If>

            <If
              when={petLetters().length > 0}
              else={
                <SunkenBox>
                  <Text class="text-xs text-muted-foreground">{tr("pet.weekly_letter_empty")}</Text>
                </SunkenBox>
              }
            >
              <View class="gap-2.5">
                <Each of={petLetters()} key={(l) => l.id}>
                  {(l) => (
                    <SunkenBox>
                      <View class="flex-row items-center justify-between gap-2">
                        <Text class="text-sm font-bold text-foreground">{l().title}</Text>
                        <VitalBadge tone="prime_ready" label={l().solar_term} />
                      </View>
                      <Text class="text-xs leading-relaxed text-foreground">{l().body}</Text>
                      <View class="flex-row items-center justify-between gap-2 pt-1">
                        <Text class="text-[11px] font-semibold text-wax-seal">
                          {l().ex_libris_seal}
                        </Text>
                        <Text class="text-[11px] text-muted-foreground">
                          {tr("pet.weekly_letter_signed", { petName: l().pet_name })}
                        </Text>
                      </View>
                    </SunkenBox>
                  )}
                </Each>
              </View>
            </If>
          </Card>
        </View>
      </If>

      {/* Subview 2: Atelier & Billing */}
      <If when={getMeSubTab() === "atelier_billing"}>
        <View class="gap-4">
          <Card>
            <SectionHeader
              title={tr("me.atelier_card_title")}
              subtitle={
                entitlements().is_atelier_unlocked
                  ? tr("me.atelier_card_active", { tier: entitlements().tier_display })
                  : tr("me.atelier_card_free")
              }
              right={
                <VitalBadge
                  tone={entitlements().is_atelier_unlocked ? "prime_ready" : "neutral"}
                  label={entitlements().tier_display}
                />
              }
            />

            <SunkenBox>
              <Text class="text-xs font-semibold text-foreground">
                {`${entitlements().tier_display} · ${entitlements().state}`}
              </Text>
              <Text class="text-xs leading-relaxed text-muted-foreground">
                {entitlements().status_banner}
              </Text>
            </SunkenBox>

            <ActionButton
              title={tr("atelier.browse_btn")}
              icon="sparkles"
              onPress={() => setShowAtelierModal(true)}
            />

            {/* Commercial Consumable Hearth Gift Pass (App Store 3.2.1(vi) compliant) */}
            <SunkenBox>
              <View class="flex-row items-center justify-between gap-2">
                <Text class="flex-1 text-xs font-bold text-foreground">
                  {tr("atelier.hearth_gift_title")}
                </Text>
                <VitalBadge tone="neutral" label={tr("atelier.hearth_gift_badge")} />
              </View>
              <Text class="text-xs leading-relaxed text-muted-foreground">
                {tr("atelier.hearth_gift_desc")}
              </Text>
              <If when={getHearthGiftCount() > 0 || entitlements().guardian_sponsor}>
                <Text class="text-[11px] font-semibold text-wax-seal">
                  {tr("atelier.hearth_gift_count", { count: Math.max(1, getHearthGiftCount()) })}
                </Text>
              </If>
              <ActionButton
                title={tr("atelier.hearth_gift_single_btn")}
                icon="hearth_ember"
                size="sm"
                variant="outline"
                onPress={() => purchaseHearthGiftPass()}
              />
            </SunkenBox>

            {/* App Store Restore Purchases & Verification */}
            <View class="flex-row flex-wrap gap-2">
              <ActionButton
                title={
                  getRestoringPurchases()
                    ? tr("atelier.compliance.restoring")
                    : tr("atelier.compliance.restore_purchases")
                }
                icon="restore_arrow"
                size="sm"
                variant="outline"
                disabled={getRestoringPurchases()}
                onPress={() => void restoreApplePurchases()}
              />
              <ActionButton
                title={tr("me.verify_ed25519_btn")}
                icon="check"
                size="sm"
                variant="outline"
                onPress={() => verifyEntitlements()}
              />
              <ActionButton
                title={tr("log.family_letter_btn")}
                icon="family_heart"
                size="sm"
                variant="secondary"
                onPress={() => generateParentsPeaceLetter()}
              />
              <If when={entitlements().is_atelier_unlocked}>
                <ActionButton
                  title={tr("me.return_free_btn")}
                  size="sm"
                  variant="ghost"
                  onPress={() => clearEntitlementToken()}
                />
              </If>
            </View>

            <If when={Boolean(getRestoreStatusMessage())}>
              <SunkenBox>
                <Text class="text-xs text-foreground">{getRestoreStatusMessage()}</Text>
              </SunkenBox>
            </If>

            <Text class="text-[11px] leading-relaxed text-muted-foreground">
              {tr("atelier.compliance.auto_renewal_disclosure")}
            </Text>
          </Card>
        </View>
      </If>

      {/* Subview 3: Vault, Privacy & Export */}
      <If when={getMeSubTab() === "vault_export"}>
        <View class="gap-4">
          <Card>
            <SectionHeader
              title={tr("me.sync_section")}
              subtitle={tr("me.sync_subtitle")}
              right={<VitalBadge tone="prime_ready" label={tr("common.encrypted")} />}
            />

            <View class="flex-row flex-wrap gap-2">
              <Each of={syncModes()} key={(m) => m.id}>
                {(m) => (
                  <SelectChip
                    label={m().label}
                    selected={settings().sync_mode === m().id}
                    onPress={() =>
                      updateSyncConfig(
                        m().id,
                        settings().webdav_url,
                        settings().byo_server_url,
                        settings().webdav_username,
                        settings().webdav_password,
                      )
                    }
                  />
                )}
              </Each>
            </View>

            <If when={settings().sync_mode === "webdav"}>
              <SunkenBox>
                <Text class="text-xs font-medium text-foreground">{tr("me.webdav_title")}</Text>
                <FieldInput
                  value={settings().webdav_url}
                  placeholder="https://dav.example.com/dav/qi-vault/"
                  onChange={(v) =>
                    updateSyncConfig(
                      "webdav",
                      v,
                      settings().byo_server_url,
                      settings().webdav_username,
                      settings().webdav_password,
                    )
                  }
                />
                <FieldInput
                  value={settings().webdav_username}
                  placeholder={tr("me.webdav_user_placeholder")}
                  onChange={(v) =>
                    updateSyncConfig(
                      "webdav",
                      settings().webdav_url,
                      settings().byo_server_url,
                      v,
                      settings().webdav_password,
                    )
                  }
                />
                <FieldInput
                  value={settings().webdav_password}
                  placeholder={tr("me.webdav_pass_placeholder")}
                  onChange={(v) =>
                    updateSyncConfig(
                      "webdav",
                      settings().webdav_url,
                      settings().byo_server_url,
                      settings().webdav_username,
                      v,
                    )
                  }
                />
                <ActionButton
                  title={tr("me.webdav_pull_btn")}
                  icon="refresh"
                  size="sm"
                  variant="outline"
                  onPress={() => pullWebdavSync()}
                />
              </SunkenBox>
            </If>

            <If when={settings().sync_mode === "byo_server"}>
              <SunkenBox>
                <Text class="text-xs font-medium text-foreground">{tr("me.byo_server_label")}</Text>
                <FieldInput
                  value={settings().byo_server_url}
                  placeholder="http://127.0.0.1:8788"
                  onChange={(v) =>
                    updateSyncConfig(
                      "byo_server",
                      settings().webdav_url,
                      v,
                      settings().webdav_username,
                      settings().webdav_password,
                    )
                  }
                />
              </SunkenBox>
            </If>

            <View class="flex-row flex-wrap gap-2">
              <ActionButton
                title={tr("me.sync_now")}
                icon="cloud_lock"
                onPress={() => runE2eeSyncNow()}
                class="flex-1"
              />
              <ActionButton
                title={tr("me.restore_btn")}
                icon="refresh"
                variant="outline"
                onPress={() => {
                  restorePhraseInput.set(settings().recovery_phrase);
                  showRestoreSheet.set(true);
                }}
              />
            </View>

            <ActionButton
              title={
                showRecoveryPhrase() ? tr("me.hide_phrase_btn") : tr("me.show_phrase_btn")
              }
              icon="user_shield"
              variant="secondary"
              onPress={() => showRecoveryPhrase.set(!showRecoveryPhrase())}
            />

            <If when={Boolean(settings().last_sync_summary)}>
              <SunkenBox>
                <Text class="text-xs font-semibold text-primary">{tr("me.sync_status_label")}</Text>
                <Text class="text-xs text-muted-foreground">{settings().last_sync_summary}</Text>
              </SunkenBox>
            </If>

            <If when={showRecoveryPhrase()}>
              <SunkenBox>
                <Text class="text-xs font-semibold text-vital-steady">
                  {tr("me.recovery_phrase_warn")}
                </Text>
                <Text class="text-sm font-semibold text-foreground">{settings().recovery_phrase}</Text>
                <ActionButton
                  title={tr("common.copy")}
                  variant="secondary"
                  onPress={() => void copyToClipboard(settings().recovery_phrase, tr("sync.passphrase_label"))}
                />
              </SunkenBox>
            </If>
          </Card>

          <Card>
            <SectionHeader title={tr("me.ai_section_title")} subtitle={tr("me.ai_section_subtitle")} />

            <View class="flex-row flex-wrap gap-2">
              <Each of={aiModes()} key={(m) => m.id}>
                {(m) => (
                  <SelectChip
                    label={m().label}
                    selected={settings().ai_mode === m().id}
                    onPress={() =>
                      updateAiConfig(
                        m().id,
                        settings().zdr_consented,
                        settings().lan_ai_url,
                        settings().lan_ai_model,
                        settings().thermal_throttled,
                        settings().simulated_ram_gb,
                      )
                    }
                  />
                )}
              </Each>
            </View>

            <If when={settings().ai_mode === "official_zdr_cloud"}>
              <SwitchRow
                title={tr("me.zdr_switch_title")}
                subtitle={tr("me.zdr_switch_sub")}
                checked={settings().zdr_consented}
                onChange={(checked) =>
                  updateAiConfig(
                    "official_zdr_cloud",
                    checked,
                    settings().lan_ai_url,
                    settings().lan_ai_model,
                    settings().thermal_throttled,
                    settings().simulated_ram_gb,
                  )
                }
              />
            </If>

            <If when={settings().ai_mode === "on_device_local"}>
              <SunkenBox>
                <Text class="text-xs leading-relaxed text-muted-foreground">
                  {tr("me.local_ai_note")}
                </Text>
              </SunkenBox>
            </If>

            <If when={settings().ai_mode === "lan_ollama"}>
              <SunkenBox>
                <Text class="text-xs font-medium text-foreground">{tr("me.ollama_title")}</Text>
                <FieldInput
                  value={settings().lan_ai_url}
                  placeholder="http://127.0.0.1:11434/v1"
                  onChange={(v) =>
                    updateAiConfig(
                      "lan_ollama",
                      settings().zdr_consented,
                      v,
                      settings().lan_ai_model,
                      settings().thermal_throttled,
                      settings().simulated_ram_gb,
                    )
                  }
                />
                <FieldInput
                  value={settings().lan_ai_model}
                  placeholder="qwen3:latest"
                  onChange={(v) =>
                    updateAiConfig(
                      "lan_ollama",
                      settings().zdr_consented,
                      settings().lan_ai_url,
                      v,
                      settings().thermal_throttled,
                      settings().simulated_ram_gb,
                    )
                  }
                />
              </SunkenBox>
            </If>
          </Card>

          <Card>
            <SectionHeader title={tr("me.push_title")} subtitle={tr("me.push_subtitle")} />
            <SwitchRow
              title={tr("me.push_switch_title", { time: settings().morning_push_time })}
              subtitle={tr("me.push_switch_sub")}
              checked={settings().morning_push_enabled}
              onChange={(checked) => toggleMorningPush(checked)}
            />
            <ActionButton
              title={tr("me.push_preview_btn")}
              icon="sun"
              variant="outline"
              onPress={() => void sendMorningTestNotification()}
            />
          </Card>

          <CollapsibleCard
            title={tr("me.cloud_card_title")}
            subtitle={tr("me.cloud_card_subtitle")}
            badgeLabel={
              ent()?.plan_tier === "pro"
                ? tr("common.pro")
                : settings().cloud_token
                  ? tr("sync.connected")
                  : tr("sync.disconnected")
            }
            badgeTone={ent()?.plan_tier === "pro" ? "prime_ready" : "neutral"}
            open={cloudAccountOpen()}
            onToggle={() => cloudAccountOpen.set(!cloudAccountOpen())}
          >
            <FieldInput
              value={settings().cloud_url}
              placeholder="http://127.0.0.1:8788"
              onChange={(v) => updateCloudUrlAndHandle(v, settings().cloud_handle)}
            />
            <FieldInput
              value={settings().cloud_handle}
              placeholder={tr("me.cloud_handle_placeholder")}
              onChange={(v) => updateCloudUrlAndHandle(settings().cloud_url, v)}
            />
            <ActionButton
              title={tr("me.cloud_connect_btn")}
              variant="outline"
              onPress={() => void connectCloudSession(settings().cloud_url, settings().cloud_handle)}
            />
            <If when={ent() !== null}>
              <SunkenBox>
                <Text class="text-xs font-semibold text-primary">
                  {`${ent()!.plan_tier.toUpperCase()} · ${ent()!.month ?? ent()!.year_month ?? "—"}`}
                </Text>
                <Text class="text-xs text-muted-foreground">
                  {ent()!.lab_parses_remaining < 0
                    ? tr("me.cloud_quota_unlimited")
                    : tr("me.cloud_quota_remaining", {
                        rem: ent()!.lab_parses_remaining,
                        used: ent()!.lab_parses_used,
                      })}
                </Text>
              </SunkenBox>
            </If>
          </CollapsibleCard>

          <Card>
            <SectionHeader title={tr("me.vault_section")} subtitle={tr("me.subtitle")} />

            <View class="flex-row flex-wrap gap-2">
              <ActionButton
                title={tr("me.export_backup")}
                variant="secondary"
                onPress={() => generateEncryptedArchive()}
                class="flex-1"
              />
              <ActionButton
                title={tr("me.export_jsonl_btn")}
                variant="secondary"
                onPress={() => generateJsonlExport()}
                class="flex-1"
              />
            </View>

            <View class="flex-row flex-wrap gap-2">
              <ActionButton
                title={tr("me.compliance_links_label")}
                variant="outline"
                onPress={() => showPrivacySheet.set(true)}
                class="flex-1"
              />
              <ActionButton
                title={tr("me.notice_btn")}
                variant="outline"
                onPress={() => showNoticeSheet.set(true)}
                class="flex-1"
              />
            </View>

            <If when={Boolean(getEncryptedArchivePreview())}>
              <SunkenBox>
                <Text class="text-xs font-semibold text-primary">{tr("me.qivault_ready")}</Text>
                <Text class="text-xs text-muted-foreground">
                  {getEncryptedArchivePreview()!.slice(0, 220)}
                </Text>
                <ActionButton
                  title={tr("common.copy")}
                  variant="secondary"
                  onPress={() =>
                    void copyToClipboard(getEncryptedArchivePreview()!, ".qivault")
                  }
                />
              </SunkenBox>
            </If>

            <If when={Boolean(getJsonlExportPreview())}>
              <SunkenBox>
                <Text class="text-xs font-semibold text-foreground">{tr("me.jsonl_ready")}</Text>
                <Text class="text-xs text-muted-foreground">{getJsonlExportPreview()!}</Text>
              </SunkenBox>
            </If>

            <ActionButton
              title={tr("me.erase_all")}
              variant="danger"
              onPress={() => showWipeConfirm.set(true)}
            />
          </Card>

          <Card>
            <View class="flex-row items-center gap-3">
              <View class="size-10 items-center justify-center rounded-xl bg-accent">
                <QiIcon name="sliders" size={20} color={token("primary")} />
              </View>
              <View class="flex-1 gap-0.5">
                <Text role="header" class="text-base font-semibold text-foreground">
                  {tr("qa.title")}
                </Text>
                <Text class="text-xs leading-relaxed text-muted-foreground">
                  {tr("qa.subtitle")}
                </Text>
              </View>
            </View>
            <ActionButton
              title={tr("me.qa_studio_btn")}
              icon="sliders"
              variant="outline"
              onPress={() => setShowQaStudio(true)}
            />
          </Card>
        </View>
      </If>

      {/* Modals shared by MeVaultPage */}
      <SheetDialog
        open={showRestoreSheet()}
        title={tr("me.restore_sheet_title")}
        onClose={() => showRestoreSheet.set(false)}
      >
        <Card>
          <Text class="text-xs leading-relaxed text-muted-foreground">
            {tr("me.restore_sheet_desc")}
          </Text>
          <View class="flex-row flex-wrap gap-2">
            <SelectChip
              label={tr("me.restore_src_icloud")}
              selected={restoreMode() === "icloud_drive"}
              onPress={() => restoreMode.set("icloud_drive")}
            />
            <SelectChip
              label={tr("me.byo_server_label")}
              selected={restoreMode() === "byo_server"}
              onPress={() => restoreMode.set("byo_server")}
            />
            <SelectChip
              label=".qivault"
              selected={restoreMode() === "archive"}
              onPress={() => restoreMode.set("archive")}
            />
          </View>
          <FieldInput
            value={restorePhraseInput()}
            placeholder={tr("sync.passphrase_placeholder")}
            onChange={(v) => restorePhraseInput.set(v)}
          />
          <FieldInput
            value={restoreSourceInput()}
            multiline={restoreMode() === "archive"}
            placeholder={
              restoreMode() === "archive"
                ? tr("me.restore_archive_placeholder")
                : tr("me.restore_url_placeholder")
            }
            onChange={(v) => restoreSourceInput.set(v)}
          />
          <ActionButton
            title={tr("me.restore_submit_btn")}
            onPress={() => {
              const ok = restoreFromRecoveryPhrase(
                restorePhraseInput(),
                restoreMode(),
                restoreSourceInput(),
              );
              if (ok) showRestoreSheet.set(false);
            }}
          />
        </Card>
      </SheetDialog>

      <SheetDialog
        open={showPrivacySheet()}
        title={tr("me.privacy_sheet_title")}
        onClose={() => showPrivacySheet.set(false)}
      >
        <Card>
          <Text class="text-sm font-semibold text-foreground">{tr("me.privacy_p1_title")}</Text>
          <Text class="text-xs leading-relaxed text-muted-foreground">
            {tr("me.privacy_p1_body")}
          </Text>
          <Text class="text-sm font-semibold text-foreground">{tr("me.privacy_p2_title")}</Text>
          <Text class="text-xs leading-relaxed text-muted-foreground">
            {tr("me.privacy_p2_body")}
          </Text>
          <Text class="text-sm font-semibold text-foreground">{tr("me.privacy_p3_title")}</Text>
          <Text class="text-xs leading-relaxed text-muted-foreground">{getDisclaimerText()}</Text>
        </Card>
      </SheetDialog>

      <SheetDialog
        open={showNoticeSheet()}
        title={tr("me.notice_btn")}
        onClose={() => showNoticeSheet.set(false)}
      >
        <Card>
          <Text class="text-sm leading-relaxed text-foreground">{getNoticeText()}</Text>
        </Card>
      </SheetDialog>

      <SheetDialog
        open={showWipeConfirm()}
        role="alertdialog"
        title={tr("me.wipe_confirm_title")}
        onClose={() => showWipeConfirm.set(false)}
      >
        <Card>
          <Text class="text-sm leading-relaxed text-foreground">{tr("me.wipe_confirm_body")}</Text>
          <View class="flex-row gap-2">
            <ActionButton
              title={tr("common.cancel")}
              variant="secondary"
              onPress={() => showWipeConfirm.set(false)}
              class="flex-1"
            />
            <ActionButton
              title={tr("me.wipe_confirm_cta")}
              variant="danger"
              onPress={() => {
                showWipeConfirm.set(false);
                wipeVaultAndReset();
              }}
              class="flex-1"
            />
          </View>
        </Card>
      </SheetDialog>
    </View>
  );
}

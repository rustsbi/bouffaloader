//! psram

use crate::{read_memory, set_bits, sleep_ms, sleep_us, write_memory};

// Base address for global control register
const GLB_BASE: u32 = 0x20000000;
// Base address for PSRAM UHS interface
const PSRAM_UHS_BASE: u32 = 0x3000f000;
// Base address for power drop sleep register
const PDS_BASE: u32 = 0x2000e000;
// Base address for PLL registers calculated from GLB_BASE
const PLL_BASE_ADDRESS: u32 = GLB_BASE + 0x7D0;

/// Initializes the Ultra High Speed (UHS) PSRAM.
/// This function configures the PLL for UHS and
/// performs the necessary register fixes for PSRAM.
pub(crate) fn uhs_psram_init() {
    glb_config_uhs_pll();
    psram_uhs_x16_init();
    fix_psram_register();
}

/// Fixes PSRAM register settings by writing predefined values
/// to specific memory addresses. These settings are necessary
/// to ensure proper initialization and functionality of
/// the Ultra High Speed (UHS) PSRAM.
fn fix_psram_register() {
    write_memory(0x2000E300, 0x1B08201B);
    write_memory(0x2000E500, 0x00000023);
    write_memory(0x2000E504, 0x00000041);

    write_memory(0x3000F000, 0xB03F0403);
    write_memory(0x3000F004, 0x81003F00);
    write_memory(0x3000F020, 0x00000023);
    write_memory(0x3000F024, 0x00000023);
    write_memory(0x3000F100, 0x60290200);
    write_memory(0x3000F104, 0x50205020);
    write_memory(0x3000F108, 0x50025002);
    write_memory(0x3000F10C, 0x50025002);
    write_memory(0x3000F110, 0x50025002);
    write_memory(0x3000F114, 0x50025002);
    write_memory(0x3000F118, 0x50025002);
    write_memory(0x3000F11C, 0x50025002);
    write_memory(0x3000F120, 0x50025002);
    write_memory(0x3000F124, 0x50025002);
    write_memory(0x3000F128, 0x34000000);
    write_memory(0x3000F12C, 0x34000006);
    // write_memory(0x3000F130, 0x0F391323);
    // write_memory(0x3000F134, 0x0B030404);
    write_memory(0x3000F138, 0x050E0418);
    write_memory(0x3000F13C, 0x0A6A1C1C);
    write_memory(0x3000F144, 0x07110710);

    write_memory(0x20000050, 0x10240408);
    write_memory(0x20000154, 0xFFFF98FF);
    write_memory(0x20000158, 0x0000FF32);
    write_memory(0x20000180, 0x03000000);
    write_memory(0x2000032C, 0x00010000);
    write_memory(0x20000330, 0x00000001);
    write_memory(0x20000334, 0x220024D0);
    write_memory(0x20000420, 0xD8000000);
    write_memory(0x20000510, 0x08000928);
    write_memory(0x20000530, 0x0000004D);
    write_memory(0x20000548, 0x01000098);
    write_memory(0x20000584, 0x9111EFF1);
    write_memory(0x20000588, 0x0FB70001);
    write_memory(0x200005C4, 0x801840EF);
    write_memory(0x2000060C, 0x000000FF);
    write_memory(0x200006C8, 0x084AB321);
    write_memory(0x200007A4, 0x00000515);
    write_memory(0x200007A8, 0x00021000);
    write_memory(0x200007D0, 0x00000725);
    write_memory(0x20000838, 0x31300434);
    write_memory(0x200008C4, 0x80401B03);
    write_memory(0x200008C8, 0x80401B03);
    write_memory(0x200008CC, 0x80401B03);
    write_memory(0x200008D0, 0x80401B03);
    write_memory(0x200008DC, 0x40401313);
    write_memory(0x200008E0, 0x40401313);
    write_memory(0x200008E4, 0x00401062);
    write_memory(0x200008F0, 0x01400B42);
    write_memory(0x200008F4, 0x00401203);
    write_memory(0x200008F8, 0x01400B42);
    write_memory(0x200008FC, 0x80400713);
    write_memory(0x20000900, 0x80400713);
    write_memory(0x20000904, 0x00401517);
    write_memory(0x20000908, 0x00401517);
    write_memory(0x20000910, 0x00401217);
    write_memory(0x2000091C, 0x10400B13);
    write_memory(0x20000920, 0x10400B13);
    write_memory(0x20000924, 0x01400B42);
    write_memory(0x20000928, 0x00401217);
    write_memory(0x20000948, 0x00401F03);
    write_memory(0x2000094C, 0xC040025A);
    write_memory(0x20000950, 0xC040025A);
    write_memory(0x20000954, 0xC040021B);
    write_memory(0x20000958, 0xC040021B);
    write_memory(0x2000095C, 0xC040021B);
    write_memory(0x20000960, 0xC040021B);
    write_memory(0x20000964, 0x01400B52);
    write_memory(0x20000968, 0x01400B52);
    write_memory(0x20000984, 0x00000003);
    write_memory(0x20000988, 0x00000003);
    write_memory(0x2000098C, 0x00000003);
    write_memory(0x20000990, 0x00000003);
    write_memory(0x20000AC4, 0x00C00000);
}

/// Configures the UHS PLL
///
/// This function first powers off the PLL, then selects the appropriate
/// clock settings, and finally powers the PLL back on to ensure it is
/// correctly configured.
fn glb_config_uhs_pll() {
    glb_power_off_pll();
    glb_clock_select_pll();
    glb_power_on_pll();
}

/// Powers off the audio PLL
///
/// This function disables the power to the audio PLL by clearing
/// specific bits in the configuration register.
fn glb_power_off_pll() {
    /* cfg0 : pu_aupll=0 */
    let mut tmp_val = read_memory(PLL_BASE_ADDRESS + 4 * 0);
    tmp_val = set_bits(tmp_val, 10, 1, 0);
    write_memory(PLL_BASE_ADDRESS + 4 * 0, tmp_val);

    /* cfg0 : pu_aupll_sfreg=0 */
    let mut tmp_val = read_memory(PLL_BASE_ADDRESS + 4 * 0);
    tmp_val = set_bits(tmp_val, 9, 1, 0);
    write_memory(PLL_BASE_ADDRESS + 4 * 0, tmp_val);
}

/// Powers on the PLL by configuring various settings including reference clock selection,
/// VCO speed, and enabling necessary features. It proceeds with setting the appropriate
/// sampling clock, configures the PLL settings, and resets to ensure proper initialization.

fn glb_power_on_pll() {
    /* cfg1:Set aupll_refclk_sel and aupll_refdiv_ratio */
    let mut tmp_val = read_memory(PLL_BASE_ADDRESS + 4 * 1);
    tmp_val = set_bits(tmp_val, 8, 4, 2);
    write_memory(PLL_BASE_ADDRESS + 4 * 1, tmp_val);

    /* cfg4:Set aupll_sel_sample_clk */
    let mut tmp_val = read_memory(PLL_BASE_ADDRESS + 4 * 4);
    tmp_val = set_bits(tmp_val, 0, 2, 2);
    write_memory(PLL_BASE_ADDRESS + 4 * 4, tmp_val);

    /* cfg5:Set aupll_vco_speed */
    let mut tmp_val = read_memory(PLL_BASE_ADDRESS + 4 * 5);
    tmp_val = set_bits(tmp_val, 0, 3, 7);
    write_memory(PLL_BASE_ADDRESS + 4 * 5, tmp_val);

    /* cfg1: uhspll_even_div_en and uhspll_even_div_ratio */
    let mut tmp_val = read_memory(PLL_BASE_ADDRESS + 4 * 1);
    tmp_val = set_bits(tmp_val, 7, 1, 1);
    tmp_val = set_bits(tmp_val, 0, 7, 2000 / 50);
    write_memory(PLL_BASE_ADDRESS + 4 * 1, tmp_val);

    /* cfg6:Set aupll_sdm_bypass,aupll_sdmin */
    let mut tmp_val = read_memory(PLL_BASE_ADDRESS + 4 * 6);
    tmp_val = set_bits(tmp_val, 0, 19, 0x32000);
    write_memory(PLL_BASE_ADDRESS + 4 * 6, tmp_val);

    /* Step2:config pu */
    /* cfg0 : pu_aupll_sfreg=1 */
    let mut tmp_val = read_memory(PLL_BASE_ADDRESS + 4 * 0);
    tmp_val = set_bits(tmp_val, 0, 19, 1);
    write_memory(PLL_BASE_ADDRESS + 4 * 0, tmp_val);

    sleep_us(3);

    /* cfg0 : pu_wifipll=1 */
    let mut tmp_val = read_memory(PLL_BASE_ADDRESS + 4 * 0);
    tmp_val = set_bits(tmp_val, 10, 1, 1);
    write_memory(PLL_BASE_ADDRESS + 4 * 0, tmp_val);

    sleep_us(3);

    /* cfg0 : aupll_sdm_reset */
    let mut tmp_val = read_memory(PLL_BASE_ADDRESS + 4 * 0);
    tmp_val = set_bits(tmp_val, 0, 1, 1);
    write_memory(PLL_BASE_ADDRESS + 4 * 0, tmp_val);
    sleep_us(2);
    let mut tmp_val = read_memory(PLL_BASE_ADDRESS + 4 * 0);
    tmp_val = set_bits(tmp_val, 0, 1, 0);
    write_memory(PLL_BASE_ADDRESS + 4 * 0, tmp_val);
    sleep_us(2);
    let mut tmp_val = read_memory(PLL_BASE_ADDRESS + 4 * 0);
    tmp_val = set_bits(tmp_val, 0, 1, 1);
    write_memory(PLL_BASE_ADDRESS + 4 * 0, tmp_val);

    /* Step3:reset pll */
    let mut tmp_val = read_memory(PLL_BASE_ADDRESS + 4 * 0);
    tmp_val = set_bits(tmp_val, 2, 1, 1);
    write_memory(PLL_BASE_ADDRESS + 4 * 0, tmp_val);
    sleep_us(2);
    let mut tmp_val = read_memory(PLL_BASE_ADDRESS + 4 * 0);
    tmp_val = set_bits(tmp_val, 2, 1, 0);
    write_memory(PLL_BASE_ADDRESS + 4 * 0, tmp_val);
    sleep_us(2);
    let mut tmp_val = read_memory(PLL_BASE_ADDRESS + 4 * 0);
    tmp_val = set_bits(tmp_val, 2, 1, 1);
    write_memory(PLL_BASE_ADDRESS + 4 * 0, tmp_val);

    sleep_us(45);
}

/// Configures the PLL reference clock selection.
///
/// This function reads a memory location associated with the PLL base address,
/// modifies specific bits to select the reference clock for the PLL, and writes
/// the updated value back to memory.
fn glb_clock_select_pll() {
    /* xxxpll_refclk_sel */
    let mut tmp_val = read_memory(PLL_BASE_ADDRESS + 4 * 1);
    tmp_val = set_bits(tmp_val, 16, 2, 0);
    write_memory(PLL_BASE_ADDRESS + 4 * 1, tmp_val);
}

/// Initializes the PSRAM UHS x16 mode.
///
/// This function performs the initialization sequence required for operating
/// the PSRAM in ultra-high-speed x16 mode. It invokes the necessary
/// sub-functions to set up both PSRAM and UHS PHY configurations.
fn psram_uhs_x16_init() {
    psram_uhs_init();
    uhs_phy_init();
}

/// This function initializes the PSRAM UHS settings.
fn psram_uhs_init() {
    write_memory(PSRAM_UHS_BASE + 0x30, 0x1a03000f);

    psram_analog_init();

    sleep_us(150);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0xC);
    tmp_val &= 0x00ffffff;
    tmp_val |= 0x04000000;
    write_memory(PSRAM_UHS_BASE + 0xC, tmp_val);

    write_memory(PSRAM_UHS_BASE + 0x10, 0x16e360);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x14);
    tmp_val = set_bits(tmp_val, 0, 16, 370);
    write_memory(PSRAM_UHS_BASE + 0x14, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x1C);
    tmp_val = set_bits(tmp_val, 0, 7, 5);
    write_memory(PSRAM_UHS_BASE + 0x1C, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x0);
    tmp_val = set_bits(tmp_val, 16, 8, 0x3f);
    tmp_val = set_bits(tmp_val, 28, 4, 0x0B);
    write_memory(PSRAM_UHS_BASE + 0x0, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x0);
    tmp_val = set_bits(tmp_val, 0, 1, 1);
    write_memory(PSRAM_UHS_BASE + 0x0, tmp_val);
}

/// This function initializes the PSRAM analog settings.
fn psram_analog_init() {
    /* power on ldo12 power_up_ldo12uhs */
    let mut tmp_val = read_memory(GLB_BASE + 0x6D0);
    tmp_val = set_bits(tmp_val, 0, 1, 1);
    write_memory(GLB_BASE + 0x6D0, tmp_val);
    sleep_us(300);
    let mut tmp_val = read_memory(GLB_BASE + 0x6D0);
    tmp_val = set_bits(tmp_val, 20, 4, 6);
    write_memory(GLB_BASE + 0x6D0, tmp_val);
    sleep_us(1);

    /* set cen ck ckn set_cen_ck_ckn */
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x150);
    tmp_val = set_bits(tmp_val, 12, 3, 0);
    tmp_val = set_bits(tmp_val, 8, 3, 0);
    write_memory(PSRAM_UHS_BASE + 0x150, tmp_val);
    sleep_us(1);
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x140);
    tmp_val &= 0xFFFCFFFF;
    tmp_val = set_bits(tmp_val, 8, 8, 1);
    write_memory(PSRAM_UHS_BASE + 0x140, tmp_val);
    sleep_us(1);

    write_memory(PSRAM_UHS_BASE + 0x100 + 0x0, 0x802b0200);
    write_memory(PSRAM_UHS_BASE + 0x100 + 0x4, 0x60206020);
    write_memory(PSRAM_UHS_BASE + 0x100 + 0x8, 0x70027002);
    write_memory(PSRAM_UHS_BASE + 0x100 + 0xC, 0x70027002);
    write_memory(PSRAM_UHS_BASE + 0x100 + 0x10, 0x70027002);
    write_memory(PSRAM_UHS_BASE + 0x100 + 0x14, 0x70027002);
    write_memory(PSRAM_UHS_BASE + 0x100 + 0x18, 0x70027002);
    write_memory(PSRAM_UHS_BASE + 0x100 + 0x1C, 0x70027002);
    write_memory(PSRAM_UHS_BASE + 0x100 + 0x20, 0x70027002);
    write_memory(PSRAM_UHS_BASE + 0x100 + 0x24, 0x70027002);
    write_memory(PSRAM_UHS_BASE + 0x100 + 0x28, 0x26000000);
    write_memory(PSRAM_UHS_BASE + 0x100 + 0x2C, 0x26000006);
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x130);
    tmp_val &= 0x08ffffff;
    tmp_val |= 0x07000000;
    write_memory(PSRAM_UHS_BASE + 0x130, tmp_val);
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x148);
    tmp_val &= 0xfffffcff;
    tmp_val |= 0x00000200;
    write_memory(PSRAM_UHS_BASE + 0x148, tmp_val);
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x14C);
    tmp_val &= 0xffe0ffff;
    write_memory(PSRAM_UHS_BASE + 0x14C, tmp_val);
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x150);
    tmp_val &= 0xff88ff88;
    tmp_val |= 0x00330033;
    write_memory(PSRAM_UHS_BASE + 0x150, tmp_val);

    sleep_us(1);

    /* switch to ldo12 switch_to_ldo12uhs */
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x140);
    tmp_val &= 0xFFCFFFFF;
    write_memory(PSRAM_UHS_BASE + 0x140, tmp_val);
    sleep_us(1);

    /* release cen ck release_cen_ck_ckn */
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x140);
    tmp_val &= 0xFFFCFFFF;
    tmp_val |= 0x30000;
    write_memory(PSRAM_UHS_BASE + 0x140, tmp_val);
    sleep_us(1);
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x150);
    tmp_val = set_bits(tmp_val, 12, 3, 3);
    tmp_val = set_bits(tmp_val, 8, 3, 3);
    write_memory(PSRAM_UHS_BASE + 0x150, tmp_val);
    sleep_us(1);

    /* config phy paramater config_uhs_phy */
    write_memory(PSRAM_UHS_BASE + 0x130, 0x1a03000f);
    write_memory(PSRAM_UHS_BASE + 0x134, 0x0b030404);
    write_memory(PSRAM_UHS_BASE + 0x138, 0x050e0419);
    write_memory(PSRAM_UHS_BASE + 0x13C, 0x0a6a1c1c);
    write_memory(PSRAM_UHS_BASE + 0x144, 0x0711070e);
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x150);
    tmp_val = set_bits(tmp_val, 24, 3, 3);
    write_memory(PSRAM_UHS_BASE + 0x150, tmp_val);
}

/// Initializes the UHS PHY interface.
/// This function configures multiple power and clock settings necessary
/// to prepare for UHS PHY initialization.
fn uhs_phy_init() {
    power_up_mm();
    power_up_uhspll();

    power_up_ldo12uhs();
    set_cen_ck_ckn();

    set_or_uhs();
    switch_to_ldo12uhs();
    release_cen_ck_ckn();

    let mut tmp_val = read_memory(GLB_BASE + 0x7F4);
    tmp_val = set_bits(tmp_val, 0, 1, 0);
    write_memory(GLB_BASE + 0x7F4, tmp_val);

    set_uhs_phy_init();
    set_uhs_latency_w();
    set_uhs_latency_r();
    psram_init();

    sleep_ms(1);
}

/// Power up the memory management module by configuring specific control registers.
///
/// This function writes to several bits in the register located at `PDS_BASE + 0x10`
/// to power up different parts of the memory management system and includes short delays
/// between operations to ensure stability.
fn power_up_mm() {
    let mut tmp_val = read_memory(PDS_BASE + 0x10);
    tmp_val = set_bits(tmp_val, 1, 1, 0);
    write_memory(PDS_BASE + 0x10, tmp_val);
    sleep_us(150);
    let mut tmp_val = read_memory(PDS_BASE + 0x10);
    tmp_val = set_bits(tmp_val, 5, 1, 0);
    write_memory(PDS_BASE + 0x10, tmp_val);
    sleep_us(10);
    let mut tmp_val = read_memory(PDS_BASE + 0x10);
    tmp_val = set_bits(tmp_val, 17, 1, 0);
    write_memory(PDS_BASE + 0x10, tmp_val);
    sleep_us(10);
    let mut tmp_val = read_memory(PDS_BASE + 0x10);
    tmp_val = set_bits(tmp_val, 13, 1, 0);
    write_memory(PDS_BASE + 0x10, tmp_val);
    sleep_us(10);
    let mut tmp_val = read_memory(PDS_BASE + 0x10);
    tmp_val = set_bits(tmp_val, 9, 1, 0);
    write_memory(PDS_BASE + 0x10, tmp_val);
    sleep_us(150);
}

/// Powers up the UHS-PLL (Ultra High-Speed Phase-Locked Loop) by configuring specific bits in the memory.
///
/// This function involves various steps where certain bits are set to specific values with intermediate delays.
/// It ensures the correct power-up sequence for the UHS-PLL by modifying the appropriate control registers.
fn power_up_uhspll() {
    let mut tmp_val = read_memory(GLB_BASE + 0x7D0);
    tmp_val = set_bits(tmp_val, 9, 1, 1);
    tmp_val = set_bits(tmp_val, 10, 1, 1);
    write_memory(GLB_BASE + 0x7D0, tmp_val);
    sleep_us(10);
    let mut tmp_val = read_memory(GLB_BASE + 0x7D0);
    tmp_val = set_bits(tmp_val, 0, 1, 0);
    write_memory(GLB_BASE + 0x7D0, tmp_val);
    sleep_us(50);
    let mut tmp_val = read_memory(GLB_BASE + 0x7D0);
    tmp_val = set_bits(tmp_val, 0, 1, 1);
    write_memory(GLB_BASE + 0x7D0, tmp_val);
    sleep_us(50);
    let mut tmp_val = read_memory(GLB_BASE + 0x7D0);
    tmp_val = set_bits(tmp_val, 2, 1, 0);
    write_memory(GLB_BASE + 0x7D0, tmp_val);
    sleep_us(50);
    let mut tmp_val = read_memory(GLB_BASE + 0x7D0);
    tmp_val = set_bits(tmp_val, 2, 1, 1);
    write_memory(GLB_BASE + 0x7D0, tmp_val);
    sleep_us(50);
}

/// Powers up the LDO12UHS (Low Dropout Regulator 12 Ultra High Speed)
/// by configuring specific bits in the memory.
///
/// This function writes to the control register at GLB_BASE + 0x6D0,
/// enabling the regulator and setting the appropriate configuration bits,
/// followed by a brief delay to ensure proper stabilization.
fn power_up_ldo12uhs() {
    let mut tmp_val = read_memory(GLB_BASE + 0x6D0);
    tmp_val = set_bits(tmp_val, 0, 1, 1);
    tmp_val = set_bits(tmp_val, 20, 4, 5);
    write_memory(GLB_BASE + 0x6D0, tmp_val);
    sleep_us(200);
}

/// Sets the CEN, CK, and CKN signals.
///
/// This function configures specific bits related to the CEN, CK, and CKN
/// signals, ensuring they are correctly set for the desired PSRAM operation.
/// It modifies the settings in the memory-mapped registers to achieve proper
/// signal configuration.
fn set_cen_ck_ckn() {
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x150);
    tmp_val = set_bits(tmp_val, 12, 3, 0);
    tmp_val = set_bits(tmp_val, 8, 3, 0);
    write_memory(PSRAM_UHS_BASE + 0x150, tmp_val);
    sleep_us(1);
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x140);
    tmp_val &= 0xFFFCFFFF;
    tmp_val = set_bits(tmp_val, 16, 8, 0xfc);
    tmp_val = set_bits(tmp_val, 8, 8, 0x1);
    write_memory(PSRAM_UHS_BASE + 0x140, tmp_val);
    sleep_us(1);
}

/// Configures various parameters for UHS settings by manipulating memory bits
/// across a range of addresses under PSRAM_UHS_BASE.
///
/// Each step involves reading the current configuration at a specific memory
/// address, adjusting specified bits to desired values using `set_bits()`
/// function, and writing back the modified values to memory. This routine
/// executes a series of such operations to ensure the UHS settings are
/// properly adjusted and concludes with a delay to ensure changes take effect.
fn set_or_uhs() {
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x148);
    tmp_val = set_bits(tmp_val, 8, 2, 2);
    write_memory(PSRAM_UHS_BASE + 0x148, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x14C);
    tmp_val = set_bits(tmp_val, 20, 1, 0);
    write_memory(PSRAM_UHS_BASE + 0x14C, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x130);
    tmp_val = set_bits(tmp_val, 28, 4, 0x0);
    tmp_val = set_bits(tmp_val, 26, 1, 1);
    write_memory(PSRAM_UHS_BASE + 0x130, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x100);
    tmp_val = set_bits(tmp_val, 28, 4, 8);
    tmp_val = set_bits(tmp_val, 16, 4, 0xb);
    write_memory(PSRAM_UHS_BASE + 0x100, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x104);
    tmp_val = set_bits(tmp_val, 28, 4, 7);
    tmp_val = set_bits(tmp_val, 12, 4, 7);
    write_memory(PSRAM_UHS_BASE + 0x104, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x124);
    tmp_val = set_bits(tmp_val, 28, 4, 7);
    tmp_val = set_bits(tmp_val, 12, 4, 7);
    write_memory(PSRAM_UHS_BASE + 0x124, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x120);
    tmp_val = set_bits(tmp_val, 28, 4, 7);
    tmp_val = set_bits(tmp_val, 12, 4, 7);
    write_memory(PSRAM_UHS_BASE + 0x120, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x11C);
    tmp_val = set_bits(tmp_val, 28, 4, 7);
    tmp_val = set_bits(tmp_val, 12, 4, 7);
    write_memory(PSRAM_UHS_BASE + 0x11C, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x118);
    tmp_val = set_bits(tmp_val, 28, 4, 7);
    tmp_val = set_bits(tmp_val, 12, 4, 7);
    write_memory(PSRAM_UHS_BASE + 0x118, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x114);
    tmp_val = set_bits(tmp_val, 28, 4, 7);
    tmp_val = set_bits(tmp_val, 12, 4, 7);
    write_memory(PSRAM_UHS_BASE + 0x114, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x110);
    tmp_val = set_bits(tmp_val, 28, 4, 7);
    tmp_val = set_bits(tmp_val, 12, 4, 7);
    write_memory(PSRAM_UHS_BASE + 0x110, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x10C);
    tmp_val = set_bits(tmp_val, 28, 4, 7);
    tmp_val = set_bits(tmp_val, 12, 4, 7);
    write_memory(PSRAM_UHS_BASE + 0x10C, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x108);
    tmp_val = set_bits(tmp_val, 28, 4, 7);
    tmp_val = set_bits(tmp_val, 12, 4, 7);
    write_memory(PSRAM_UHS_BASE + 0x108, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x128);
    tmp_val = set_bits(tmp_val, 24, 4, 6);
    write_memory(PSRAM_UHS_BASE + 0x128, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x12C);
    tmp_val = set_bits(tmp_val, 24, 4, 6);
    write_memory(PSRAM_UHS_BASE + 0x12C, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x130);
    tmp_val = set_bits(tmp_val, 24, 2, 3);
    write_memory(PSRAM_UHS_BASE + 0x130, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x100);
    tmp_val = set_bits(tmp_val, 20, 2, 2);
    tmp_val = set_bits(tmp_val, 8, 2, 2);
    write_memory(PSRAM_UHS_BASE + 0x100, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x104);
    tmp_val = set_bits(tmp_val, 20, 2, 2);
    tmp_val = set_bits(tmp_val, 4, 2, 2);
    write_memory(PSRAM_UHS_BASE + 0x104, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x124);
    tmp_val = set_bits(tmp_val, 0, 2, 2);
    tmp_val = set_bits(tmp_val, 16, 2, 2);
    write_memory(PSRAM_UHS_BASE + 0x124, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x120);
    tmp_val = set_bits(tmp_val, 0, 2, 2);
    tmp_val = set_bits(tmp_val, 16, 2, 2);
    write_memory(PSRAM_UHS_BASE + 0x120, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x11C);
    tmp_val = set_bits(tmp_val, 0, 2, 2);
    tmp_val = set_bits(tmp_val, 16, 2, 2);
    write_memory(PSRAM_UHS_BASE + 0x11C, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x118);
    tmp_val = set_bits(tmp_val, 0, 2, 2);
    tmp_val = set_bits(tmp_val, 16, 2, 2);
    write_memory(PSRAM_UHS_BASE + 0x118, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x114);
    tmp_val = set_bits(tmp_val, 0, 2, 2);
    tmp_val = set_bits(tmp_val, 16, 2, 2);
    write_memory(PSRAM_UHS_BASE + 0x114, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x110);
    tmp_val = set_bits(tmp_val, 0, 2, 2);
    tmp_val = set_bits(tmp_val, 16, 2, 2);
    write_memory(PSRAM_UHS_BASE + 0x110, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x10C);
    tmp_val = set_bits(tmp_val, 0, 2, 2);
    tmp_val = set_bits(tmp_val, 16, 2, 2);
    write_memory(PSRAM_UHS_BASE + 0x10C, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x108);
    tmp_val = set_bits(tmp_val, 0, 2, 2);
    tmp_val = set_bits(tmp_val, 16, 2, 2);
    write_memory(PSRAM_UHS_BASE + 0x108, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x150);
    tmp_val = set_bits(tmp_val, 16, 3, 3);
    tmp_val = set_bits(tmp_val, 20, 3, 3);
    tmp_val = set_bits(tmp_val, 0, 3, 3);
    tmp_val = set_bits(tmp_val, 4, 3, 3);
    write_memory(PSRAM_UHS_BASE + 0x150, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x124);
    tmp_val = set_bits(tmp_val, 24, 4, 0);
    tmp_val = set_bits(tmp_val, 8, 4, 0);
    write_memory(PSRAM_UHS_BASE + 0x124, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x120);
    tmp_val = set_bits(tmp_val, 24, 4, 0);
    tmp_val = set_bits(tmp_val, 8, 4, 0);
    write_memory(PSRAM_UHS_BASE + 0x120, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x11c);
    tmp_val = set_bits(tmp_val, 24, 4, 0);
    tmp_val = set_bits(tmp_val, 8, 4, 0);
    write_memory(PSRAM_UHS_BASE + 0x11c, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x118);
    tmp_val = set_bits(tmp_val, 24, 4, 0);
    tmp_val = set_bits(tmp_val, 8, 4, 0);
    write_memory(PSRAM_UHS_BASE + 0x118, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x114);
    tmp_val = set_bits(tmp_val, 24, 4, 0);
    tmp_val = set_bits(tmp_val, 8, 4, 0);
    write_memory(PSRAM_UHS_BASE + 0x114, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x110);
    tmp_val = set_bits(tmp_val, 24, 4, 0);
    tmp_val = set_bits(tmp_val, 8, 4, 0);
    write_memory(PSRAM_UHS_BASE + 0x110, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x10c);
    tmp_val = set_bits(tmp_val, 24, 4, 0);
    tmp_val = set_bits(tmp_val, 8, 4, 0);
    write_memory(PSRAM_UHS_BASE + 0x10c, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x108);
    tmp_val = set_bits(tmp_val, 24, 4, 0);
    tmp_val = set_bits(tmp_val, 8, 4, 0);
    write_memory(PSRAM_UHS_BASE + 0x108, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x128);
    tmp_val = set_bits(tmp_val, 24, 4, 0);
    tmp_val = set_bits(tmp_val, 8, 4, 0);
    tmp_val = set_bits(tmp_val, 28, 4, 3);
    write_memory(PSRAM_UHS_BASE + 0x128, tmp_val);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x12C);
    tmp_val = set_bits(tmp_val, 24, 4, 0);
    tmp_val = set_bits(tmp_val, 8, 4, 0);
    tmp_val = set_bits(tmp_val, 28, 4, 3);
    write_memory(PSRAM_UHS_BASE + 0x12C, tmp_val);

    sleep_us(200);
}

/// Switches the power supply to LDO12UHS mode by configuring the relevant bits.
///
/// This function reads the current configuration from the memory address
/// at `PSRAM_UHS_BASE + 0x140`, sets specific bits to configure the power supply
/// to LDO12UHS mode, writes the updated configuration back to the same address,
/// and then waits briefly to ensure the change takes effect.
fn switch_to_ldo12uhs() {
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x140);
    tmp_val = set_bits(tmp_val, 16, 8, 0xcc);
    write_memory(PSRAM_UHS_BASE + 0x140, tmp_val);
    sleep_us(200);
}

/// Releases the CEN and CK signals.
///
/// This function reads and modifies the memory configuration at two
/// specific addresses to release the CEN and CK signal configurations.
/// It writes the changes back to these addresses and includes short
/// delays to allow the changes to take effect.
fn release_cen_ck_ckn() {
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x140);
    tmp_val = set_bits(tmp_val, 16, 8, 0xcf);
    tmp_val = set_bits(tmp_val, 8, 8, 0x0);
    write_memory(PSRAM_UHS_BASE + 0x140, tmp_val);
    sleep_us(10);
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x150);
    tmp_val = set_bits(tmp_val, 8, 3, 3);
    tmp_val = set_bits(tmp_val, 12, 3, 3);
    write_memory(PSRAM_UHS_BASE + 0x150, tmp_val);
    sleep_us(10);
}

/// Initializes the PHY settings for UHS mode.
///
/// This function reads and sets specific bits in several memory addresses related
/// to PHY initialization under the UHS settings. It ensures the correct configuration
/// of these bits to establish proper PHY operation before proceeding with UHS mode.
fn set_uhs_phy_init() {
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x130);
    tmp_val = set_bits(tmp_val, 28, 4, 0x0);
    tmp_val = set_bits(tmp_val, 27, 1, 1);
    tmp_val = set_bits(tmp_val, 26, 1, 1);
    tmp_val = set_bits(tmp_val, 24, 2, 3);
    write_memory(PSRAM_UHS_BASE + 0x130, tmp_val);

    write_memory(PSRAM_UHS_BASE + 0x134, 0x09020303);
    write_memory(PSRAM_UHS_BASE + 0x138, 0x040c0313);
    write_memory(PSRAM_UHS_BASE + 0x13C, 0x07d11515);
    write_memory(PSRAM_UHS_BASE + 0x144, 0x060f050c);

    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x150);
    tmp_val = set_bits(tmp_val, 24, 3, 1);
    write_memory(PSRAM_UHS_BASE + 0x150, tmp_val);

    sleep_us(100);
}

/// Sets the write latency for UHS (Ultra High Speed) mode.
///
/// This function configures the write latency for UHS mode by manipulating
/// specific bits in the memory address located at `PSRAM_UHS_BASE + 0x130`.
/// The latency is represented in terms of a base latency value with incremental
/// adjustments, ensuring the system operates efficiently under UHS conditions.
fn set_uhs_latency_w() {
    let uhs_latency = 9;
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x130);
    tmp_val = set_bits(tmp_val, 12, 3, uhs_latency % 4);
    tmp_val = set_bits(tmp_val, 8, 3, uhs_latency / 4);
    tmp_val = set_bits(tmp_val, 4, 3, (uhs_latency + 1) % 4);
    tmp_val = set_bits(tmp_val, 0, 3, (uhs_latency + 1) / 4);
    write_memory(PSRAM_UHS_BASE + 0x130, tmp_val);
    sleep_us(50);
}

/// Sets the read latency for UHS (Ultra High Speed) mode.
///
/// This function configures the read latency for UHS mode
/// by manipulating specific bits in the memory address located
/// at `PSRAM_UHS_BASE + 0x130`. The latency is represented
/// by dividing the total latency into smaller bit segments to
/// ensure efficient system operation under UHS conditions.
fn set_uhs_latency_r() {
    let uhs_latency = 30;
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x130);
    tmp_val = set_bits(tmp_val, 20, 3, uhs_latency % 4);
    tmp_val = set_bits(tmp_val, 16, 4, uhs_latency / 4);
    write_memory(PSRAM_UHS_BASE + 0x130, tmp_val);
    sleep_us(50);
}

/// Initializes the PSRAM.
///
/// This function sets up the initial configuration of PSRAM by adjusting
/// specific memory bits to enable the required settings. It includes
/// setting a bit high, a brief delay, followed by another bit high.
/// Finally, it resets the first bit with another delay.
fn psram_init() {
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x0);
    tmp_val = set_bits(tmp_val, 2, 1, 1);
    write_memory(PSRAM_UHS_BASE + 0x0, tmp_val);
    sleep_us(10);
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x4);
    tmp_val = set_bits(tmp_val, 0, 1, 1);
    write_memory(PSRAM_UHS_BASE + 0x4, tmp_val);
    sleep_us(100);
    let mut tmp_val = read_memory(PSRAM_UHS_BASE + 0x0);
    tmp_val = set_bits(tmp_val, 2, 1, 0);
    write_memory(PSRAM_UHS_BASE + 0x0, tmp_val);
    sleep_us(100);
}

use super::*;

#[derive(Debug, Parser)]
pub(crate) struct Inscriptions {
  #[clap(long, help = "Maximum number of inscriptions to list")]
  limit: Option<usize>,
  #[clap(long, help = "Maximum inscription number to list")]
  max_number: Option<i64>,
  #[clap(long, help = "Maximum inscription block height to list")]
  max_height: Option<u64>,
  #[clap(long, help = "Maximum sat number to list")]
  max_sat: Option<Sat>,
  #[clap(long, help = "Specific single inscription number to show")]
  number: Option<i64>,
  #[clap(long, help = "Specific single inscription id to show")]
  id: Option<InscriptionId>,
  #[clap(long, help = "Only list inscriptions on uncommon sats or rarer.")]
  uncommon: bool,
  #[clap(
    long,
    help = "List inscriptions in order of inscribed satoshi ordinals."
  )]
  order_by_sat: bool,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputWithSatWithAddress {
  pub sat: Option<Sat>,
  pub number: i64,
  pub height: u64,
  pub timestamp: u32,
  pub inscription: InscriptionId,
  pub location: SatPoint,
  pub address: Address,
  pub amount: u64,
  pub content_type: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputWithoutSatWithAddress {
  pub number: i64,
  pub height: u64,
  pub timestamp: u32,
  pub inscription: InscriptionId,
  pub location: SatPoint,
  pub address: Address,
  pub amount: u64,
  pub content_type: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputWithSat {
  pub sat: Option<Sat>,
  pub number: i64,
  pub height: u64,
  pub timestamp: u32,
  pub inscription: InscriptionId,
  pub location: SatPoint,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputWithoutSat {
  pub number: i64,
  pub height: u64,
  pub timestamp: u32,
  pub inscription: InscriptionId,
  pub location: SatPoint,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputUnbound {
  pub number: i64,
  pub height: u64,
  pub timestamp: u32,
  pub inscription: InscriptionId,
  pub location: SatPoint,
  pub content_type: String,
}

impl Inscriptions {
  pub(crate) fn run(self, options: Options) -> Result {
    let index = Index::open(&options)?;

    index.update()?;

    let index_has_sats = index.has_sat_index()?;

    if !index_has_sats {
      if self.max_sat.is_some() {
        bail!("--max-sat requires index created with `--index-sats` flag")
      }

      if self.uncommon {
        bail!("--uncommon requires index created with `--index-sats` flag")
      }
    }

    if self.number.is_some() && self.id.is_some() {
      bail!("can't specify --number and --id");
    }

    if self.number.is_some() || self.id.is_some() {
      let inscription = if self.number.is_some() {
        let number = self.number.unwrap();
        index
          .get_inscription_id_by_inscription_number(number)?
          .ok_or_else(|| anyhow!("Inscription {number} not found"))?
      } else {
        self.id.unwrap()
      };

      let entry = index.get_inscription_entry(inscription)?.ok_or_else(|| anyhow!("Inscription {inscription} not found"))?;
      let location = index.get_inscription_satpoint_by_id(inscription)?.unwrap();
      let content_type = index
        .get_inscription_by_id(inscription)?
        .ok_or_else(|| anyhow!("inscription {inscription} not found"))?
        .content_type()
        .unwrap()
        .to_string();

      if format!("{}", location.outpoint.txid) == "0000000000000000000000000000000000000000000000000000000000000000" {
        print_json(OutputUnbound {
          inscription,
          location,
          number: entry.number,
          height: entry.height,
          timestamp: entry.timestamp,
          content_type,
        })?;
      } else {
        let output = index
          .get_transaction(location.outpoint.txid)?
          .unwrap()
          .output
          .into_iter()
          .nth(location.outpoint.vout.try_into().unwrap())
          .unwrap();
        let amount = output.value;
        let address = options.chain().address_from_script(&output.script_pubkey)?;
        if index_has_sats {
          print_json(OutputWithSatWithAddress {
            sat: entry.sat,
            inscription,
            location,
            number: entry.number,
            height: entry.height,
            timestamp: entry.timestamp,
            address,
            amount,
            content_type,
          })?;
        } else {
          print_json(OutputWithoutSatWithAddress {
            inscription,
            location,
            number: entry.number,
            height: entry.height,
            timestamp: entry.timestamp,
            address,
            amount,
            content_type,
          })?;
        }
      }
      return Ok(());
    }

    let inscriptions = if self.order_by_sat {
      index.get_inscriptions_by_sat(
        self.limit,
        self.max_number,
        self.max_height,
        self.max_sat,
        self.uncommon,
      )?
    } else {
      index.get_inscriptions_by_inscription_number(
        self.limit,
        self.max_number,
        self.max_height,
        self.max_sat,
        self.uncommon,
      )?
    };

    let mut output_with_sat = Vec::new();
    let mut output_without_sat = Vec::new();

    for inscription in inscriptions {
      let entry = index
        .get_inscription_entry(inscription)?
        .ok_or_else(|| anyhow!("Inscription {inscription} not found"))?;
      let location = index.get_inscription_satpoint_by_id(inscription)?.unwrap();
      if index_has_sats {
        output_with_sat.push(OutputWithSat {
          sat: entry.sat,
          inscription,
          location,
          number: entry.number,
          height: entry.height,
          timestamp: entry.timestamp,
        });
      } else {
        output_without_sat.push(OutputWithoutSat {
          inscription,
          location,
          number: entry.number,
          height: entry.height,
          timestamp: entry.timestamp,
        });
      }
    }

    if index_has_sats {
      print_json(&output_with_sat)?;
    } else {
      print_json(&output_without_sat)?;
    }

    Ok(())
  }
}

/*
 * Created on Tue Jul 23 2023
 *
 * This file is a part of Skytable
 * Skytable (formerly known as TerrabaseDB or Skybase) is a free and open-source
 * NoSQL database written by Sayan Nandan ("the Author") with the
 * vision to provide flexibility in data modelling without compromising
 * on performance, queryability or scalability.
 *
 * Copyright (c) 2023, Sayan Nandan <ohsayan@outlook.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 *
*/

use {
    super::{
        header_impl::{
            FileScope, FileSpecifier, FileSpecifierVersion, HostRunMode, SDSSHeader, SDSSHeaderRaw,
        },
        SDSSResult,
    },
    crate::engine::storage::v1::SDSSError,
    std::{
        fs::File,
        io::{Read, Seek, SeekFrom, Write},
    },
};

#[derive(Debug)]
/// Log whether
pub enum FileOpen<F> {
    Created(F),
    Existing(F, SDSSHeader),
}

#[derive(Debug)]
pub enum RawFileOpen<F> {
    Created(F),
    Existing(F),
}

pub trait RawFileIOInterface: Sized {
    fn fopen_or_create_rw(file_path: &str) -> SDSSResult<RawFileOpen<Self>>;
    fn fread_exact(&mut self, buf: &mut [u8]) -> SDSSResult<()>;
    fn fwrite_all(&mut self, bytes: &[u8]) -> SDSSResult<()>;
    fn fsync_all(&mut self) -> SDSSResult<()>;
    fn fseek_ahead(&mut self, by: u64) -> SDSSResult<()>;
    fn flen(&self) -> SDSSResult<u64>;
}

impl RawFileIOInterface for File {
    fn fopen_or_create_rw(file_path: &str) -> SDSSResult<RawFileOpen<Self>> {
        let f = File::options()
            .create(true)
            .read(true)
            .write(true)
            .open(file_path)?;
        let md = f.metadata()?;
        if md.created()? == md.modified()? {
            return Ok(RawFileOpen::Created(f));
        } else {
            return Ok(RawFileOpen::Existing(f));
        }
    }
    fn fread_exact(&mut self, buf: &mut [u8]) -> SDSSResult<()> {
        self.read_exact(buf)?;
        Ok(())
    }
    fn fwrite_all(&mut self, bytes: &[u8]) -> SDSSResult<()> {
        self.write_all(bytes)?;
        Ok(())
    }
    fn fsync_all(&mut self) -> SDSSResult<()> {
        self.sync_all()?;
        Ok(())
    }
    fn flen(&self) -> SDSSResult<u64> {
        Ok(self.metadata()?.len())
    }
    fn fseek_ahead(&mut self, by: u64) -> SDSSResult<()> {
        self.seek(SeekFrom::Start(by))?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SDSSFileIO<F> {
    f: F,
}

impl<F: RawFileIOInterface> SDSSFileIO<F> {
    pub fn open_or_create_perm_rw(
        file_path: &str,
        file_scope: FileScope,
        file_specifier: FileSpecifier,
        file_specifier_version: FileSpecifierVersion,
        host_setting_version: u32,
        host_run_mode: HostRunMode,
        host_startup_counter: u64,
    ) -> SDSSResult<FileOpen<Self>> {
        let f = F::fopen_or_create_rw(file_path)?;
        match f {
            RawFileOpen::Created(f) => {
                // since this file was just created, we need to append the header
                let data = SDSSHeaderRaw::new_auto(
                    file_scope,
                    file_specifier,
                    file_specifier_version,
                    host_setting_version,
                    host_run_mode,
                    host_startup_counter,
                    0,
                )
                .array();
                let mut f = Self::_new(f);
                f.fsynced_write(&data)?;
                Ok(FileOpen::Created(f))
            }
            RawFileOpen::Existing(mut f) => {
                // this is an existing file. decoded the header
                let mut header_raw = [0u8; SDSSHeaderRaw::header_size()];
                f.fread_exact(&mut header_raw)?;
                let header =
                    SDSSHeaderRaw::decode_noverify(header_raw).ok_or(SDSSError::CorruptedHeader)?;
                // now validate the header
                header.verify(file_scope, file_specifier, file_specifier_version)?;
                // since we updated this file, let us update the header
                let mut new_header = header.clone();
                new_header.dr_rs_mut().bump_modify_count();
                let mut f = Self::_new(f);
                f.fsynced_write(new_header.encoded().array().as_ref())?;
                Ok(FileOpen::Existing(f, header))
            }
        }
    }
}

impl<F: RawFileIOInterface> SDSSFileIO<F> {
    fn _new(f: F) -> Self {
        Self { f }
    }
    pub fn unfsynced_write(&mut self, data: &[u8]) -> SDSSResult<()> {
        self.f.fwrite_all(data)
    }
    pub fn fsync_all(&mut self) -> SDSSResult<()> {
        self.f.fsync_all()?;
        Ok(())
    }
    pub fn fsynced_write(&mut self, data: &[u8]) -> SDSSResult<()> {
        self.f.fwrite_all(data)?;
        self.f.fsync_all()
    }
    pub fn read_to_buffer(&mut self, buffer: &mut [u8]) -> SDSSResult<()> {
        self.f.fread_exact(buffer)
    }
    pub fn file_length(&self) -> SDSSResult<u64> {
        self.f.flen()
    }
    pub fn seek_ahead(&mut self, by: u64) -> SDSSResult<()> {
        self.f.fseek_ahead(by)
    }
}

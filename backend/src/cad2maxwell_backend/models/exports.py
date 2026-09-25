from pydantic import BaseModel, Field


class ExportRequest(BaseModel):
    import_id: str = Field(min_length=1)
    section_id: str = Field(min_length=1)
